use capitainerie_common::{ClientId, CoreError, IntegrationInstanceId};

use super::{
    BaseUrl, ConnectorFailure, Environment, InstanceConnection, InstanceError, InstanceName,
    IntegrationInstance, IntegrationKind, ProbeReport, clean_odoo_field,
    ports::IntegrationInstanceRepository, secret_preview, stamp,
};
use crate::domain::{
    agent::SecretToken,
    authorization::{AccessRequest, Action, ResourceKind, Subject, ports::Authorizer, require},
    client::ports::ClientRepository,
};

pub const DATABASE_MAX_CHARS: usize = 63;
pub const LOGIN_MAX_CHARS: usize = 254;
pub const SECRET_MAX_CHARS: usize = 4096;
pub const INSTANCE_IN_USE: &str = "instance_in_use";

const NEW_SECRET_REQUIRED: &str =
    "a new secret is required when the address, the database or the login changes";

#[derive(Debug, Clone)]
pub struct CreateInstance {
    pub client_id: ClientId,
    pub kind: String,
    pub name: String,
    pub environment: String,
    pub base_url: String,
    pub database: Option<String>,
    pub login: Option<String>,
    pub secret: SecretToken,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateInstance {
    pub name: Option<String>,
    pub environment: Option<String>,
    pub base_url: Option<String>,
    pub database: Option<String>,
    pub login: Option<String>,
    pub secret: Option<SecretToken>,
}

#[derive(Debug)]
pub struct PreparedProbe {
    pub instance: IntegrationInstance,
    pub connection: InstanceConnection,
}

#[derive(Debug)]
pub struct PreparedUpdate {
    pub instance: IntegrationInstance,
    pub probe: Option<InstanceConnection>,
    pub new_secret: bool,
}

impl PreparedUpdate {
    fn sealed_secret(&self) -> Option<&SecretToken> {
        self.probe
            .as_ref()
            .filter(|_| self.new_secret)
            .map(|connection| &connection.secret)
    }
}

pub struct IntegrationInstanceService<'a, I, C, A> {
    instances: I,
    clients: C,
    authorizer: &'a A,
}

impl<'a, I, C, A> IntegrationInstanceService<'a, I, C, A>
where
    I: IntegrationInstanceRepository,
    C: ClientRepository,
    A: Authorizer,
{
    pub fn new(instances: I, clients: C, authorizer: &'a A) -> Self {
        Self {
            instances,
            clients,
            authorizer,
        }
    }

    pub async fn list(
        mut self,
        subject: Subject,
        client_id: ClientId,
        include_archived: bool,
    ) -> Result<Vec<IntegrationInstance>, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::IntegrationRead, None).await?;
        self.clients
            .find(organization_id, client_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        self.instances
            .list_for_client(organization_id, client_id, include_archived)
            .await
    }

    pub async fn prepare_create(
        mut self,
        subject: Subject,
        command: CreateInstance,
    ) -> Result<PreparedProbe, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject.clone(), Action::IntegrationManage, None)
            .await?;
        let kind = IntegrationKind::try_from(command.kind.as_str())?;
        let name = InstanceName::try_from(command.name)?;
        let environment = Environment::try_from(command.environment.as_str())?;
        let base_url = BaseUrl::try_from(command.base_url)?;
        let (database, login) =
            credentials(kind, present(command.database), present(command.login))?;
        checked_secret(&command.secret)?;
        let client = self
            .clients
            .find(organization_id, command.client_id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if client.is_archived() {
            return Err(CoreError::Invalid(format!(
                "the client {} is archived",
                client.name.as_str()
            )));
        }
        let now = stamp();
        let instance = IntegrationInstance {
            id: IntegrationInstanceId::new(),
            organization_id,
            client_id: client.id,
            kind,
            name,
            environment,
            base_url,
            database,
            login,
            secret_preview: secret_preview(command.secret.expose()),
            server_version: None,
            checked_at: None,
            created_by: subject.id,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };
        let connection = instance.connection(command.secret);
        Ok(PreparedProbe {
            instance,
            connection,
        })
    }

    pub async fn save_create(
        mut self,
        prepared: PreparedProbe,
        report: ProbeReport,
    ) -> Result<IntegrationInstance, InstanceError> {
        let PreparedProbe {
            mut instance,
            connection,
        } = prepared;
        instance.server_version = Some(report.server_version);
        instance.checked_at = Some(stamp());
        self.instances
            .insert(&instance, &connection.secret)
            .await
            .map_err(|error| written(error, &instance.name))?;
        Ok(instance)
    }

    pub async fn prepare_update(
        mut self,
        subject: Subject,
        id: IntegrationInstanceId,
        command: UpdateInstance,
    ) -> Result<PreparedUpdate, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::IntegrationManage, Some(id))
            .await?;
        let mut instance = self
            .instances
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        refuse_archived(&instance)?;
        if command.name.is_none()
            && command.environment.is_none()
            && command.base_url.is_none()
            && command.database.is_none()
            && command.login.is_none()
            && command.secret.is_none()
        {
            return Err(CoreError::Invalid(
                "nothing to update: give a name, environment, URL, database, login or secret"
                    .into(),
            ));
        }
        let mut needs_probe = false;
        let mut leaves_credentials = false;
        if let Some(name) = command.name {
            instance.name = InstanceName::try_from(name)?;
        }
        if let Some(environment) = command.environment {
            instance.environment = Environment::try_from(environment.as_str())?;
        }
        if let Some(base_url) = command.base_url {
            let base_url = BaseUrl::try_from(base_url)?;
            needs_probe |= base_url != instance.base_url;
            leaves_credentials |= !same_origin(&base_url, &instance.base_url);
            instance.base_url = base_url;
        }
        let database_changed = replace(&mut instance.database, command.database);
        let login_changed = replace(&mut instance.login, command.login);
        needs_probe |= database_changed || login_changed;
        leaves_credentials |= database_changed || login_changed;
        let (database, login) = credentials(
            instance.kind,
            instance.database.take(),
            instance.login.take(),
        )?;
        instance.database = database;
        instance.login = login;
        instance.updated_at = stamp();
        let new_secret = command.secret.is_some();
        let secret = match command.secret {
            Some(secret) => {
                checked_secret(&secret)?;
                instance.secret_preview = secret_preview(secret.expose());
                Some(secret)
            }
            None if leaves_credentials => {
                return Err(CoreError::Invalid(NEW_SECRET_REQUIRED.into()));
            }
            None if needs_probe => Some(
                self.instances
                    .connection(organization_id, id)
                    .await?
                    .ok_or(CoreError::NotFound)?
                    .secret,
            ),
            None => None,
        };
        let probe = secret.map(|secret| instance.connection(secret));
        Ok(PreparedUpdate {
            instance,
            probe,
            new_secret,
        })
    }

    pub async fn save_update(
        mut self,
        mut prepared: PreparedUpdate,
        report: Option<ProbeReport>,
    ) -> Result<IntegrationInstance, InstanceError> {
        if let Some(report) = report {
            prepared.instance.server_version = Some(report.server_version);
            prepared.instance.checked_at = Some(stamp());
        }
        self.instances
            .update(&prepared.instance, prepared.sealed_secret())
            .await
            .map_err(|error| written(error, &prepared.instance.name))?;
        Ok(prepared.instance)
    }

    pub async fn prepare_check(
        mut self,
        subject: Subject,
        id: IntegrationInstanceId,
    ) -> Result<PreparedProbe, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::IntegrationManage, Some(id))
            .await?;
        let instance = self
            .instances
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        refuse_archived(&instance)?;
        let connection = self
            .instances
            .connection(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        Ok(PreparedProbe {
            instance,
            connection,
        })
    }

    pub async fn record_check(
        mut self,
        mut instance: IntegrationInstance,
        outcome: &Result<ProbeReport, ConnectorFailure>,
    ) -> Result<IntegrationInstance, CoreError> {
        instance.checked_at = Some(stamp());
        if let Ok(report) = outcome {
            instance.server_version = Some(report.server_version.clone());
        }
        self.instances.update(&instance, None).await?;
        Ok(instance)
    }

    pub async fn archive(
        mut self,
        subject: Subject,
        id: IntegrationInstanceId,
        held_by_an_intervention: bool,
    ) -> Result<IntegrationInstance, CoreError> {
        let organization_id = subject.organization_id;
        self.require(subject, Action::IntegrationManage, Some(id))
            .await?;
        let mut instance = self
            .instances
            .find(organization_id, id)
            .await?
            .ok_or(CoreError::NotFound)?;
        if instance.is_archived() {
            return Ok(instance);
        }
        if held_by_an_intervention {
            return Err(CoreError::Conflict(INSTANCE_IN_USE.into()));
        }
        let now = stamp();
        instance.archived_at = Some(now);
        instance.updated_at = now;
        self.instances.update(&instance, None).await?;
        Ok(instance)
    }

    async fn require(
        &self,
        subject: Subject,
        action: Action,
        id: Option<IntegrationInstanceId>,
    ) -> Result<(), CoreError> {
        require(
            self.authorizer,
            AccessRequest::new(
                subject,
                action,
                ResourceKind::IntegrationInstance,
                id.map(|id| id.0),
            ),
        )
        .await
    }
}

fn written(error: CoreError, name: &InstanceName) -> InstanceError {
    match error {
        CoreError::Conflict(_) => InstanceError::NameTaken(name.as_str().to_owned()),
        other => InstanceError::Core(other),
    }
}

fn refuse_archived(instance: &IntegrationInstance) -> Result<(), CoreError> {
    if instance.is_archived() {
        return Err(CoreError::Invalid(format!(
            "the instance {} is archived",
            instance.name.as_str()
        )));
    }
    Ok(())
}

fn same_origin(left: &BaseUrl, right: &BaseUrl) -> bool {
    left.is_https() == right.is_https()
        && left.host().eq_ignore_ascii_case(right.host())
        && port_of(left) == port_of(right)
}

fn port_of(url: &BaseUrl) -> u16 {
    let default = if url.is_https() { 443 } else { 80 };
    let Some((_, rest)) = url.as_str().split_once("://") else {
        return default;
    };
    let authority = rest.split('/').next().unwrap_or(rest);
    authority
        .get(url.host().len()..)
        .and_then(|port| port.strip_prefix(':'))
        .and_then(|port| port.parse().ok())
        .unwrap_or(default)
}

fn present(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn replace(current: &mut Option<String>, incoming: Option<String>) -> bool {
    let Some(incoming) = incoming else {
        return false;
    };
    let incoming = present(Some(incoming));
    let changed = incoming != *current;
    *current = incoming;
    changed
}

fn credentials(
    kind: IntegrationKind,
    database: Option<String>,
    login: Option<String>,
) -> Result<(Option<String>, Option<String>), CoreError> {
    match kind {
        IntegrationKind::OdooJsonRpc => {
            let database = database.ok_or_else(|| {
                CoreError::Invalid("the database is required for Odoo 18 and earlier".into())
            })?;
            let login = login.ok_or_else(|| {
                CoreError::Invalid("the login is required for Odoo 18 and earlier".into())
            })?;
            Ok((
                Some(clean_odoo_field(
                    database,
                    DATABASE_MAX_CHARS,
                    "the database",
                )?),
                Some(clean_odoo_field(login, LOGIN_MAX_CHARS, "the login")?),
            ))
        }
        IntegrationKind::OdooJson2 => {
            if login.is_some() {
                return Err(CoreError::Invalid(
                    "the login is only used with Odoo 18 and earlier".into(),
                ));
            }
            let database = database
                .map(|database| clean_odoo_field(database, DATABASE_MAX_CHARS, "the database"))
                .transpose()?;
            Ok((database, None))
        }
        IntegrationKind::N8n => {
            if database.is_some() || login.is_some() {
                return Err(CoreError::Invalid(
                    "n8n takes neither a database nor a login".into(),
                ));
            }
            Ok((None, None))
        }
    }
}

fn checked_secret(secret: &SecretToken) -> Result<(), CoreError> {
    let value = secret.expose();
    if value.trim().is_empty() {
        return Err(CoreError::Invalid("the secret is empty".into()));
    }
    if value.chars().count() > SECRET_MAX_CHARS {
        return Err(CoreError::Invalid(format!(
            "the secret exceeds {SECRET_MAX_CHARS} characters"
        )));
    }
    if value.chars().any(char::is_control) {
        return Err(CoreError::Invalid(
            "the secret contains a control character".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use capitainerie_common::OrganizationId;

    use chrono::Utc;

    use super::*;
    use crate::{
        domain::{
            authorization::{Decision, ports::MockAuthorizer},
            client::{Client, links::tests::client, ports::MockClientRepository},
            integration::ports::MockIntegrationInstanceRepository,
        },
        infrastructure::authorization::LocalPolicyEngine,
    };

    const KEY: &str = "sk-0123456789abcdef-wxyz";

    fn subject(organization_id: OrganizationId, is_admin: bool) -> Subject {
        Subject {
            id: "user-1".into(),
            organization_id,
            actor: None,
            is_admin,
        }
    }

    fn allow() -> MockAuthorizer {
        let mut authorizer = MockAuthorizer::new();
        authorizer
            .expect_evaluate()
            .returning(|_| Box::pin(async { Ok(Decision::Allow) }));
        authorizer
    }

    fn deny() -> MockAuthorizer {
        let mut authorizer = MockAuthorizer::new();
        authorizer.expect_evaluate().returning(|_| {
            Box::pin(async {
                Ok(Decision::Deny {
                    reason: "no".into(),
                })
            })
        });
        authorizer
    }

    fn holding(client: &Client) -> MockClientRepository {
        let client = client.clone();
        let mut clients = MockClientRepository::new();
        clients.expect_find().returning(move |_, _| {
            let client = client.clone();
            Box::pin(async move { Ok(Some(client)) })
        });
        clients
    }

    fn command(client: &Client) -> CreateInstance {
        CreateInstance {
            client_id: client.id,
            kind: "odoo_json2".into(),
            name: "Odoo prod".into(),
            environment: "production".into(),
            base_url: "https://odoo.acme.test/".into(),
            database: None,
            login: None,
            secret: SecretToken::new(KEY),
        }
    }

    fn stored(
        organization_id: OrganizationId,
        client_id: ClientId,
        kind: IntegrationKind,
    ) -> IntegrationInstance {
        let (database, login) = match kind {
            IntegrationKind::OdooJsonRpc => (Some("acme".to_owned()), Some("api".to_owned())),
            _ => (None, None),
        };
        IntegrationInstance {
            id: IntegrationInstanceId::new(),
            organization_id,
            client_id,
            kind,
            name: InstanceName::try_from("Odoo prod".to_owned()).unwrap(),
            environment: Environment::Production,
            base_url: BaseUrl::try_from("https://odoo.acme.test".to_owned()).unwrap(),
            database,
            login,
            secret_preview: "…wxyz".into(),
            server_version: Some("19.0".into()),
            checked_at: Some(Utc::now()),
            created_by: "user-1".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            archived_at: None,
        }
    }

    fn finding(instance: &IntegrationInstance) -> MockIntegrationInstanceRepository {
        let instance = instance.clone();
        let mut instances = MockIntegrationInstanceRepository::new();
        instances.expect_find().returning(move |_, _| {
            let instance = instance.clone();
            Box::pin(async move { Ok(Some(instance)) })
        });
        instances
    }

    fn with_stored_connection(
        instances: &mut MockIntegrationInstanceRepository,
        instance: &IntegrationInstance,
    ) {
        let connection = instance.connection(SecretToken::new("stored-secret-value"));
        instances.expect_connection().returning(move |_, _| {
            let connection = connection.clone();
            Box::pin(async move { Ok(Some(connection)) })
        });
    }

    fn report() -> ProbeReport {
        ProbeReport {
            server_version: "19.0".into(),
            warning: None,
        }
    }

    fn service<'a>(
        instances: MockIntegrationInstanceRepository,
        clients: MockClientRepository,
        authorizer: &'a MockAuthorizer,
    ) -> IntegrationInstanceService<
        'a,
        MockIntegrationInstanceRepository,
        MockClientRepository,
        MockAuthorizer,
    > {
        IntegrationInstanceService::new(instances, clients, authorizer)
    }

    #[tokio::test]
    async fn a_denied_caller_reaches_neither_the_instances_nor_the_clients() {
        let authorizer = deny();
        let organization_id = OrganizationId::new();
        let acme = client(organization_id, "Acme");
        let id = IntegrationInstanceId::new();
        let who = subject(organization_id, true);
        let fresh = || {
            service(
                MockIntegrationInstanceRepository::new(),
                MockClientRepository::new(),
                &authorizer,
            )
        };

        let outcomes = [
            fresh().list(who.clone(), acme.id, false).await.map(|_| ()),
            fresh()
                .prepare_create(who.clone(), command(&acme))
                .await
                .map(|_| ()),
            fresh()
                .prepare_update(
                    who.clone(),
                    id,
                    UpdateInstance {
                        name: Some("x".into()),
                        ..UpdateInstance::default()
                    },
                )
                .await
                .map(|_| ()),
            fresh().prepare_check(who.clone(), id).await.map(|_| ()),
            fresh().archive(who, id, false).await.map(|_| ()),
        ];

        for outcome in outcomes {
            assert!(matches!(outcome, Err(CoreError::Forbidden { .. })));
        }
    }

    #[tokio::test]
    async fn only_an_admin_manages_instances_while_a_member_reads_them() {
        let organization_id = OrganizationId::new();
        let acme = client(organization_id, "Acme");
        let member = subject(organization_id, false);
        let authorizer = LocalPolicyEngine;
        let id = IntegrationInstanceId::new();

        let created = IntegrationInstanceService::new(
            MockIntegrationInstanceRepository::new(),
            MockClientRepository::new(),
            &authorizer,
        )
        .prepare_create(member.clone(), command(&acme))
        .await;
        let archived = IntegrationInstanceService::new(
            MockIntegrationInstanceRepository::new(),
            MockClientRepository::new(),
            &authorizer,
        )
        .archive(member.clone(), id, false)
        .await;
        let mut instances = MockIntegrationInstanceRepository::new();
        let listed_instance = stored(organization_id, acme.id, IntegrationKind::N8n);
        instances
            .expect_list_for_client()
            .returning(move |_, _, _| {
                let listed = listed_instance.clone();
                Box::pin(async move { Ok(vec![listed]) })
            });
        let listed = IntegrationInstanceService::new(instances, holding(&acme), &authorizer)
            .list(member, acme.id, false)
            .await
            .unwrap();

        assert!(matches!(created, Err(CoreError::Forbidden { .. })));
        assert!(matches!(archived, Err(CoreError::Forbidden { .. })));
        assert_eq!(listed.len(), 1);
    }

    #[tokio::test]
    async fn listing_an_unknown_client_is_not_found() {
        let authorizer = allow();
        let mut clients = MockClientRepository::new();
        clients
            .expect_find()
            .returning(|_, _| Box::pin(async { Ok(None) }));

        let error = service(
            MockIntegrationInstanceRepository::new(),
            clients,
            &authorizer,
        )
        .list(
            subject(OrganizationId::new(), false),
            ClientId::new(),
            false,
        )
        .await
        .unwrap_err();

        assert!(matches!(error, CoreError::NotFound));
    }

    #[tokio::test]
    async fn a_valid_instance_is_prepared_for_a_probe_with_nothing_stored() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let acme = client(organization_id, "Acme");

        let prepared = service(
            MockIntegrationInstanceRepository::new(),
            holding(&acme),
            &authorizer,
        )
        .prepare_create(
            subject(organization_id, true),
            CreateInstance {
                name: "  Odoo prod ".into(),
                database: Some("  ".into()),
                ..command(&acme)
            },
        )
        .await
        .unwrap();

        assert_eq!(prepared.instance.name.as_str(), "Odoo prod");
        assert_eq!(
            prepared.instance.base_url.as_str(),
            "https://odoo.acme.test"
        );
        assert_eq!(prepared.instance.client_id, acme.id);
        assert_eq!(prepared.instance.organization_id, organization_id);
        assert_eq!(prepared.instance.created_by, "user-1");
        assert_eq!(prepared.instance.secret_preview, "…wxyz");
        assert_eq!(prepared.instance.database, None);
        assert_eq!(prepared.instance.server_version, None);
        assert_eq!(prepared.instance.checked_at, None);
        assert_eq!(prepared.connection.instance_id, prepared.instance.id);
        assert_eq!(prepared.connection.kind, IntegrationKind::OdooJson2);
        assert_eq!(prepared.connection.secret.expose(), KEY);
        assert!(!format!("{prepared:?}").contains(KEY));
    }

    #[tokio::test]
    async fn the_credentials_each_kind_needs_are_enforced() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let acme = client(organization_id, "Acme");
        let with = |kind: &str, database: Option<&str>, login: Option<&str>| CreateInstance {
            kind: kind.into(),
            database: database.map(str::to_owned),
            login: login.map(str::to_owned),
            ..command(&acme)
        };
        let refused = [
            with("odoo_jsonrpc", None, None),
            with("odoo_jsonrpc", Some("acme"), None),
            with("odoo_jsonrpc", None, Some("api")),
            with(
                "odoo_jsonrpc",
                Some(&"d".repeat(DATABASE_MAX_CHARS + 1)),
                Some("api"),
            ),
            with(
                "odoo_jsonrpc",
                Some("acme"),
                Some(&"l".repeat(LOGIN_MAX_CHARS + 1)),
            ),
            with("odoo_jsonrpc", Some("ac\nme"), Some("api")),
            with("odoo_json2", None, Some("api")),
            with(
                "odoo_json2",
                Some(&"d".repeat(DATABASE_MAX_CHARS + 1)),
                None,
            ),
            with("n8n", Some("acme"), None),
            with("n8n", None, Some("api")),
            with("odoo", None, None),
        ];
        let accepted = [
            with("odoo_jsonrpc", Some("acme"), Some("api")),
            with("odoo_json2", None, None),
            with("odoo_json2", Some("acme"), None),
            with("n8n", None, None),
            with("n8n", Some(" "), Some("")),
        ];

        for command in refused {
            let label = format!(
                "{} {:?} {:?}",
                command.kind, command.database, command.login
            );
            let error = service(
                MockIntegrationInstanceRepository::new(),
                holding(&acme),
                &authorizer,
            )
            .prepare_create(subject(organization_id, true), command)
            .await
            .unwrap_err();
            assert!(matches!(error, CoreError::Invalid(_)), "{label}");
        }
        for command in accepted {
            let label = format!(
                "{} {:?} {:?}",
                command.kind, command.database, command.login
            );
            service(
                MockIntegrationInstanceRepository::new(),
                holding(&acme),
                &authorizer,
            )
            .prepare_create(subject(organization_id, true), command)
            .await
            .unwrap_or_else(|error| panic!("{label}: {error}"));
        }
    }

    #[tokio::test]
    async fn a_bad_url_environment_name_or_secret_is_refused_before_the_client_is_read() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let acme = client(organization_id, "Acme");
        let refused = [
            CreateInstance {
                base_url: "ftp://odoo.acme.test".into(),
                ..command(&acme)
            },
            CreateInstance {
                base_url: "https://user:pass@odoo.acme.test".into(),
                ..command(&acme)
            },
            CreateInstance {
                environment: "prod".into(),
                ..command(&acme)
            },
            CreateInstance {
                name: " ".into(),
                ..command(&acme)
            },
            CreateInstance {
                name: "n".repeat(InstanceName::MAX_CHARS + 1),
                ..command(&acme)
            },
            CreateInstance {
                secret: SecretToken::new(""),
                ..command(&acme)
            },
            CreateInstance {
                secret: SecretToken::new("   "),
                ..command(&acme)
            },
            CreateInstance {
                secret: SecretToken::new("sk-abc\ndef"),
                ..command(&acme)
            },
            CreateInstance {
                secret: SecretToken::new("k".repeat(SECRET_MAX_CHARS + 1)),
                ..command(&acme)
            },
        ];

        for command in refused {
            let error = service(
                MockIntegrationInstanceRepository::new(),
                MockClientRepository::new(),
                &authorizer,
            )
            .prepare_create(subject(organization_id, true), command)
            .await
            .unwrap_err();
            assert!(matches!(error, CoreError::Invalid(_)));
        }
    }

    #[tokio::test]
    async fn an_unknown_or_archived_client_takes_no_instance() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let mut archived = client(organization_id, "Acme");
        archived.archived_at = Some(Utc::now());
        let mut unknown = MockClientRepository::new();
        unknown
            .expect_find()
            .returning(|_, _| Box::pin(async { Ok(None) }));

        let on_archived = service(
            MockIntegrationInstanceRepository::new(),
            holding(&archived),
            &authorizer,
        )
        .prepare_create(subject(organization_id, true), command(&archived))
        .await
        .unwrap_err();
        let on_unknown = service(
            MockIntegrationInstanceRepository::new(),
            unknown,
            &authorizer,
        )
        .prepare_create(subject(organization_id, true), command(&archived))
        .await
        .unwrap_err();

        assert!(matches!(on_archived, CoreError::Invalid(_)));
        assert!(matches!(on_unknown, CoreError::NotFound));
    }

    #[tokio::test]
    async fn a_probed_instance_is_stored_with_the_version_and_the_date_of_the_probe() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let acme = client(organization_id, "Acme");
        let prepared = service(
            MockIntegrationInstanceRepository::new(),
            holding(&acme),
            &authorizer,
        )
        .prepare_create(subject(organization_id, true), command(&acme))
        .await
        .unwrap();
        let mut instances = MockIntegrationInstanceRepository::new();
        instances
            .expect_insert()
            .times(1)
            .withf(|instance, secret| {
                instance.server_version.as_deref() == Some("19.0")
                    && instance.checked_at.is_some()
                    && secret.expose() == KEY
            })
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let saved = service(instances, MockClientRepository::new(), &authorizer)
            .save_create(prepared, report())
            .await
            .unwrap();

        assert_eq!(saved.server_version.as_deref(), Some("19.0"));
        assert!(saved.checked_at.is_some());
    }

    #[tokio::test]
    async fn a_name_already_taken_by_the_client_is_reported_with_that_name() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let acme = client(organization_id, "Acme");
        let prepared = service(
            MockIntegrationInstanceRepository::new(),
            holding(&acme),
            &authorizer,
        )
        .prepare_create(subject(organization_id, true), command(&acme))
        .await
        .unwrap();
        let mut instances = MockIntegrationInstanceRepository::new();
        instances
            .expect_insert()
            .returning(|_, _| Box::pin(async { Err(CoreError::Conflict("duplicate key".into())) }));

        let error = service(instances, MockClientRepository::new(), &authorizer)
            .save_create(prepared, report())
            .await
            .unwrap_err();

        assert!(matches!(error, InstanceError::NameTaken(name) if name == "Odoo prod"));
    }

    #[tokio::test]
    async fn a_failing_store_is_not_mistaken_for_a_taken_name() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let acme = client(organization_id, "Acme");
        let prepared = service(
            MockIntegrationInstanceRepository::new(),
            holding(&acme),
            &authorizer,
        )
        .prepare_create(subject(organization_id, true), command(&acme))
        .await
        .unwrap();
        let mut instances = MockIntegrationInstanceRepository::new();
        instances
            .expect_insert()
            .returning(|_, _| Box::pin(async { Err(CoreError::Database("down".into())) }));

        let error = service(instances, MockClientRepository::new(), &authorizer)
            .save_create(prepared, report())
            .await
            .unwrap_err();

        assert!(matches!(error, InstanceError::Core(CoreError::Database(_))));
    }

    async fn updated(
        kind: IntegrationKind,
        command: UpdateInstance,
    ) -> Result<PreparedUpdate, CoreError> {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let instance = stored(organization_id, ClientId::new(), kind);
        let mut instances = finding(&instance);
        with_stored_connection(&mut instances, &instance);
        service(instances, MockClientRepository::new(), &authorizer)
            .prepare_update(subject(organization_id, true), instance.id, command)
            .await
    }

    #[tokio::test]
    async fn a_name_or_environment_update_is_not_probed() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let instance = stored(organization_id, ClientId::new(), IntegrationKind::OdooJson2);

        let prepared = service(finding(&instance), MockClientRepository::new(), &authorizer)
            .prepare_update(
                subject(organization_id, true),
                instance.id,
                UpdateInstance {
                    name: Some(" Odoo recette ".into()),
                    environment: Some("staging".into()),
                    base_url: Some("https://odoo.acme.test/".into()),
                    ..UpdateInstance::default()
                },
            )
            .await
            .unwrap();

        assert!(prepared.probe.is_none());
        assert!(!prepared.new_secret);
        assert_eq!(prepared.instance.name.as_str(), "Odoo recette");
        assert_eq!(prepared.instance.environment, Environment::Staging);
        assert!(prepared.instance.updated_at > instance.updated_at);
    }

    #[tokio::test]
    async fn a_change_of_path_alone_is_probed_with_the_stored_secret() {
        for base_url in [
            "https://odoo.acme.test/odoo",
            "HTTPS://ODOO.ACME.TEST:443/",
            "https://odoo.acme.test:443",
        ] {
            let prepared = updated(
                IntegrationKind::OdooJson2,
                UpdateInstance {
                    base_url: Some(base_url.into()),
                    ..UpdateInstance::default()
                },
            )
            .await
            .unwrap();

            let probe = prepared.probe.as_ref().unwrap();
            assert_eq!(probe.secret.expose(), "stored-secret-value", "{base_url}");
            assert!(!prepared.new_secret);
            assert!(prepared.sealed_secret().is_none());
        }
    }

    async fn refused_without_a_secret(kind: IntegrationKind, command: UpdateInstance) -> CoreError {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let instance = stored(organization_id, ClientId::new(), kind);
        service(finding(&instance), MockClientRepository::new(), &authorizer)
            .prepare_update(subject(organization_id, true), instance.id, command)
            .await
            .unwrap_err()
    }

    #[tokio::test]
    async fn a_new_origin_database_or_login_needs_a_new_secret_and_never_reads_the_stored_one() {
        let cases = [
            (
                IntegrationKind::OdooJson2,
                UpdateInstance {
                    base_url: Some("https://odoo2.acme.test".into()),
                    ..UpdateInstance::default()
                },
            ),
            (
                IntegrationKind::OdooJson2,
                UpdateInstance {
                    base_url: Some("http://odoo.acme.test".into()),
                    ..UpdateInstance::default()
                },
            ),
            (
                IntegrationKind::OdooJson2,
                UpdateInstance {
                    base_url: Some("https://odoo.acme.test:8443/odoo".into()),
                    ..UpdateInstance::default()
                },
            ),
            (
                IntegrationKind::OdooJson2,
                UpdateInstance {
                    database: Some("acme".into()),
                    ..UpdateInstance::default()
                },
            ),
            (
                IntegrationKind::OdooJsonRpc,
                UpdateInstance {
                    database: Some("other".into()),
                    ..UpdateInstance::default()
                },
            ),
            (
                IntegrationKind::OdooJsonRpc,
                UpdateInstance {
                    login: Some("robot".into()),
                    ..UpdateInstance::default()
                },
            ),
            (
                IntegrationKind::OdooJsonRpc,
                UpdateInstance {
                    name: Some("Odoo recette".into()),
                    base_url: Some("https://elsewhere.test".into()),
                    login: Some("robot".into()),
                    ..UpdateInstance::default()
                },
            ),
        ];

        for (kind, command) in cases {
            let label = format!("{kind:?} {command:?}");
            let error = refused_without_a_secret(kind, command).await;
            assert!(
                matches!(&error, CoreError::Invalid(message) if message == NEW_SECRET_REQUIRED),
                "{label}: {error:?}"
            );
        }
    }

    #[tokio::test]
    async fn a_new_origin_database_or_login_is_probed_with_the_new_secret() {
        let new_key = || Some(SecretToken::new("brand-new-key-1234"));
        let moved = updated(
            IntegrationKind::OdooJson2,
            UpdateInstance {
                base_url: Some("https://odoo2.acme.test".into()),
                secret: new_key(),
                ..UpdateInstance::default()
            },
        )
        .await
        .unwrap();
        let probe = moved.probe.as_ref().unwrap();
        assert_eq!(probe.base_url.as_str(), "https://odoo2.acme.test");
        assert_eq!(probe.secret.expose(), "brand-new-key-1234");
        assert!(moved.new_secret);

        let database = updated(
            IntegrationKind::OdooJson2,
            UpdateInstance {
                database: Some("acme".into()),
                secret: new_key(),
                ..UpdateInstance::default()
            },
        )
        .await
        .unwrap();
        let probe = database.probe.as_ref().unwrap();
        assert_eq!(probe.database.as_deref(), Some("acme"));
        assert_eq!(probe.secret.expose(), "brand-new-key-1234");

        let login = updated(
            IntegrationKind::OdooJsonRpc,
            UpdateInstance {
                login: Some("robot".into()),
                secret: new_key(),
                ..UpdateInstance::default()
            },
        )
        .await
        .unwrap();
        let probe = login.probe.as_ref().unwrap();
        assert_eq!(probe.login.as_deref(), Some("robot"));
        assert_eq!(probe.secret.expose(), "brand-new-key-1234");
    }

    #[tokio::test]
    async fn the_same_database_and_login_do_not_ask_for_a_secret() {
        let prepared = updated(
            IntegrationKind::OdooJsonRpc,
            UpdateInstance {
                database: Some(" acme ".into()),
                login: Some("api".into()),
                ..UpdateInstance::default()
            },
        )
        .await
        .unwrap();

        assert!(prepared.probe.is_none());
        assert!(!prepared.new_secret);
    }

    #[tokio::test]
    async fn a_new_secret_is_probed_and_replaces_the_stored_one() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let instance = stored(organization_id, ClientId::new(), IntegrationKind::N8n);

        let prepared = service(finding(&instance), MockClientRepository::new(), &authorizer)
            .prepare_update(
                subject(organization_id, true),
                instance.id,
                UpdateInstance {
                    secret: Some(SecretToken::new("brand-new-key-1234")),
                    ..UpdateInstance::default()
                },
            )
            .await
            .unwrap();

        assert!(prepared.new_secret);
        assert_eq!(
            prepared.sealed_secret().unwrap().expose(),
            "brand-new-key-1234"
        );
        assert_eq!(prepared.instance.secret_preview, "…1234");
        assert_eq!(
            prepared.probe.as_ref().unwrap().secret.expose(),
            "brand-new-key-1234"
        );
    }

    #[tokio::test]
    async fn a_saved_update_writes_the_secret_only_when_it_changed() {
        let authorizer = allow();
        let url_only = updated(
            IntegrationKind::OdooJson2,
            UpdateInstance {
                base_url: Some("https://odoo.acme.test/odoo".into()),
                ..UpdateInstance::default()
            },
        )
        .await
        .unwrap();
        let mut keeps = MockIntegrationInstanceRepository::new();
        keeps
            .expect_update()
            .times(1)
            .withf(|instance, secret| {
                secret.is_none() && instance.server_version.as_deref() == Some("19.0")
            })
            .returning(|_, _| Box::pin(async { Ok(()) }));

        service(keeps, MockClientRepository::new(), &authorizer)
            .save_update(url_only, Some(report()))
            .await
            .unwrap();

        let replaced = updated(
            IntegrationKind::N8n,
            UpdateInstance {
                secret: Some(SecretToken::new("brand-new-key-1234")),
                ..UpdateInstance::default()
            },
        )
        .await
        .unwrap();
        let mut replaces = MockIntegrationInstanceRepository::new();
        replaces
            .expect_update()
            .times(1)
            .withf(|_, secret| secret.is_some_and(|secret| secret.expose() == "brand-new-key-1234"))
            .returning(|_, _| Box::pin(async { Ok(()) }));

        service(replaces, MockClientRepository::new(), &authorizer)
            .save_update(replaced, Some(report()))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn an_update_renaming_to_a_taken_name_is_reported() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let instance = stored(organization_id, ClientId::new(), IntegrationKind::N8n);
        let prepared = service(finding(&instance), MockClientRepository::new(), &authorizer)
            .prepare_update(
                subject(organization_id, true),
                instance.id,
                UpdateInstance {
                    name: Some("n8n prod".into()),
                    ..UpdateInstance::default()
                },
            )
            .await
            .unwrap();
        let mut instances = MockIntegrationInstanceRepository::new();
        instances
            .expect_update()
            .returning(|_, _| Box::pin(async { Err(CoreError::Conflict("duplicate".into())) }));

        let error = service(instances, MockClientRepository::new(), &authorizer)
            .save_update(prepared, None)
            .await
            .unwrap_err();

        assert!(matches!(error, InstanceError::NameTaken(name) if name == "n8n prod"));
    }

    #[tokio::test]
    async fn an_update_cannot_break_the_credentials_of_its_kind() {
        for (kind, command) in [
            (
                IntegrationKind::OdooJsonRpc,
                UpdateInstance {
                    database: Some(" ".into()),
                    ..UpdateInstance::default()
                },
            ),
            (
                IntegrationKind::OdooJson2,
                UpdateInstance {
                    login: Some("robot".into()),
                    ..UpdateInstance::default()
                },
            ),
            (
                IntegrationKind::N8n,
                UpdateInstance {
                    database: Some("acme".into()),
                    ..UpdateInstance::default()
                },
            ),
            (
                IntegrationKind::N8n,
                UpdateInstance {
                    secret: Some(SecretToken::new("")),
                    ..UpdateInstance::default()
                },
            ),
            (
                IntegrationKind::N8n,
                UpdateInstance {
                    base_url: Some("https://u:p@n8n.acme.test".into()),
                    ..UpdateInstance::default()
                },
            ),
            (IntegrationKind::N8n, UpdateInstance::default()),
        ] {
            let error = updated(kind, command).await.unwrap_err();
            assert!(matches!(error, CoreError::Invalid(_)));
        }
    }

    #[tokio::test]
    async fn a_blank_database_clears_it_for_json2() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let mut instance = stored(organization_id, ClientId::new(), IntegrationKind::OdooJson2);
        instance.database = Some("acme".into());

        let prepared = service(finding(&instance), MockClientRepository::new(), &authorizer)
            .prepare_update(
                subject(organization_id, true),
                instance.id,
                UpdateInstance {
                    database: Some("".into()),
                    secret: Some(SecretToken::new("brand-new-key-1234")),
                    ..UpdateInstance::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(prepared.instance.database, None);
        assert!(prepared.probe.is_some());
    }

    #[tokio::test]
    async fn an_unknown_or_archived_instance_cannot_be_updated_or_checked() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let mut archived = stored(organization_id, ClientId::new(), IntegrationKind::N8n);
        archived.archived_at = Some(Utc::now());
        let mut unknown = MockIntegrationInstanceRepository::new();
        unknown
            .expect_find()
            .returning(|_, _| Box::pin(async { Ok(None) }));
        let rename = || UpdateInstance {
            name: Some("x".into()),
            ..UpdateInstance::default()
        };

        let update_archived = service(finding(&archived), MockClientRepository::new(), &authorizer)
            .prepare_update(subject(organization_id, true), archived.id, rename())
            .await
            .unwrap_err();
        let check_archived = service(finding(&archived), MockClientRepository::new(), &authorizer)
            .prepare_check(subject(organization_id, true), archived.id)
            .await
            .unwrap_err();
        let check_unknown = service(unknown, MockClientRepository::new(), &authorizer)
            .prepare_check(subject(organization_id, true), archived.id)
            .await
            .unwrap_err();

        assert!(matches!(update_archived, CoreError::Invalid(_)));
        assert!(matches!(check_archived, CoreError::Invalid(_)));
        assert!(matches!(check_unknown, CoreError::NotFound));
    }

    #[tokio::test]
    async fn a_check_probes_the_stored_connection_and_records_the_outcome() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let instance = stored(organization_id, ClientId::new(), IntegrationKind::OdooJson2);
        let mut instances = finding(&instance);
        with_stored_connection(&mut instances, &instance);

        let prepared = service(instances, MockClientRepository::new(), &authorizer)
            .prepare_check(subject(organization_id, true), instance.id)
            .await
            .unwrap();
        assert_eq!(prepared.connection.secret.expose(), "stored-secret-value");

        let mut succeeded = MockIntegrationInstanceRepository::new();
        succeeded
            .expect_update()
            .times(1)
            .withf(|instance, secret| {
                secret.is_none() && instance.server_version.as_deref() == Some("19.0+e")
            })
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let recorded = service(succeeded, MockClientRepository::new(), &authorizer)
            .record_check(
                prepared.instance.clone(),
                &Ok(ProbeReport {
                    server_version: "19.0+e".into(),
                    warning: None,
                }),
            )
            .await
            .unwrap();
        assert_eq!(recorded.server_version.as_deref(), Some("19.0+e"));

        let mut failed = MockIntegrationInstanceRepository::new();
        failed
            .expect_update()
            .times(1)
            .withf(|instance, _| instance.server_version.as_deref() == Some("19.0"))
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let before = prepared.instance.checked_at;
        let recorded = service(failed, MockClientRepository::new(), &authorizer)
            .record_check(prepared.instance, &Err(ConnectorFailure::Unauthorized))
            .await
            .unwrap();
        assert_eq!(recorded.server_version.as_deref(), Some("19.0"));
        assert!(recorded.checked_at > before);
    }

    #[tokio::test]
    async fn archiving_sets_the_dates_once() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let instance = stored(organization_id, ClientId::new(), IntegrationKind::N8n);
        let mut instances = finding(&instance);
        instances
            .expect_update()
            .times(1)
            .withf(|instance, secret| instance.archived_at.is_some() && secret.is_none())
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let archived = service(instances, MockClientRepository::new(), &authorizer)
            .archive(subject(organization_id, true), instance.id, false)
            .await
            .unwrap();
        assert!(archived.is_archived());
        assert_eq!(archived.archived_at, Some(archived.updated_at));

        let again = service(finding(&archived), MockClientRepository::new(), &authorizer)
            .archive(subject(organization_id, true), instance.id, false)
            .await
            .unwrap();
        assert_eq!(again.archived_at, archived.archived_at);
    }

    #[tokio::test]
    async fn an_instance_held_by_an_open_intervention_is_not_archived() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let instance = stored(organization_id, ClientId::new(), IntegrationKind::N8n);

        let error = service(finding(&instance), MockClientRepository::new(), &authorizer)
            .archive(subject(organization_id, true), instance.id, true)
            .await
            .unwrap_err();

        assert!(matches!(error, CoreError::Conflict(code) if code == INSTANCE_IN_USE));
    }

    #[tokio::test]
    async fn an_archived_instance_stays_archived_whatever_holds_it() {
        let authorizer = allow();
        let organization_id = OrganizationId::new();
        let mut instance = stored(organization_id, ClientId::new(), IntegrationKind::N8n);
        instance.archived_at = Some(Utc::now());

        let again = service(finding(&instance), MockClientRepository::new(), &authorizer)
            .archive(subject(organization_id, true), instance.id, true)
            .await
            .unwrap();

        assert_eq!(again, instance);
    }

    #[tokio::test]
    async fn a_denied_caller_learns_nothing_about_an_instance_held_by_an_intervention() {
        let authorizer = deny();
        let organization_id = OrganizationId::new();

        let error = service(
            MockIntegrationInstanceRepository::new(),
            MockClientRepository::new(),
            &authorizer,
        )
        .archive(
            subject(organization_id, false),
            IntegrationInstanceId::new(),
            true,
        )
        .await
        .unwrap_err();

        assert!(matches!(error, CoreError::Forbidden { .. }));
    }
}
