use capitainerie_common::IntegrationActionId;
use chrono::{DateTime, Utc};
use serde_json::Value;

use super::{
    ConnectorFailure, IntegrationInstance,
    action::{ActionCall, ActionRequest},
    journal::{
        ActionContext, ActionOutcome, Denial, IntegrationAction, Outcome, ensure_granted,
        is_sensitive_model,
    },
    redact::{MASK, for_model, redact},
};
use crate::domain::authorization::Decision;

pub const BEFORE_UNREADABLE: &str = "before_unreadable";

pub const REASON_MAX_CHARS: usize = 2000;

const TARGET_KEYS: [&str; 8] = [
    "model",
    "workflow_id",
    "execution_id",
    "workflow_name",
    "tag_name",
    "tag_id",
    "attachment_id",
    "resource",
];

pub fn ensure_allowed(
    decision: &Decision,
    context: &ActionContext,
    instance: Option<&IntegrationInstance>,
    call: &ActionCall,
) -> Result<(), Denial> {
    if let Decision::Deny { reason } = decision {
        return Err(Denial::PolicyDenied(reason.clone()));
    }
    let instance = instance
        .filter(|instance| {
            instance.id == call.instance
                && instance.organization_id == context.subject.organization_id
        })
        .ok_or(Denial::InstanceNotGranted)?;
    if instance.is_archived() {
        return Err(Denial::InstanceArchived);
    }
    if instance.client_id != context.client_id {
        return Err(Denial::InstanceOfAnotherClient);
    }
    ensure_granted(&context.grants, instance.id, call.request.access())?;
    if call.request.family() != instance.kind.family() {
        return Err(Denial::ToolNotForInstance);
    }
    match call.request.odoo_model() {
        Some(model) if is_sensitive_model(model) => Err(Denial::SensitiveModel(model.to_owned())),
        _ => Ok(()),
    }
}

pub fn publication_guard(
    instance: &IntegrationInstance,
    request: &ActionRequest,
    before: Option<&Value>,
) -> Result<(), Denial> {
    let published = before
        .and_then(|workflow| workflow.get("active"))
        .and_then(Value::as_bool)
        == Some(true);
    match request {
        ActionRequest::N8nUpdateWorkflow { publish: false, .. }
            if published && instance.server_version.as_deref() == Some("1") =>
        {
            Err(Denial::NotSupported(
                "n8n 1.x would publish this change at once: unpublish the workflow first or send publish: true"
                    .into(),
            ))
        }
        _ => Ok(()),
    }
}

pub fn fill_from_before(request: &mut ActionRequest, before: Option<&Value>) {
    let ActionRequest::N8nUpdateWorkflow { name, settings, .. } = request else {
        return;
    };
    if name.is_none() {
        *name = before
            .and_then(|workflow| workflow.get("name"))
            .and_then(Value::as_str)
            .map(str::to_owned);
    }
    if settings.is_none() {
        *settings = Some(
            before
                .and_then(|workflow| workflow.get("settings"))
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default(),
        );
    }
}

fn scrub_text(text: &str, secret: &str) -> String {
    if secret.is_empty() {
        text.to_owned()
    } else {
        text.replace(secret, MASK)
    }
}

fn scrub_value(value: Value, secret: &str) -> Value {
    match value {
        Value::String(text) => Value::String(scrub_text(&text, secret)),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| scrub_value(item, secret))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| (key, scrub_value(value, secret)))
                .collect(),
        ),
        other => other,
    }
}

fn target_label(request: &ActionRequest) -> Option<String> {
    let target = request.target();
    TARGET_KEYS.iter().find_map(|key| match target.get(*key)? {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    })
}

pub fn unrecorded(denial: &Denial, request: &ActionRequest) -> ActionOutcome {
    let code = denial.code();
    let target = target_label(request)
        .map(|label| format!(" {label}"))
        .unwrap_or_default();
    ActionOutcome {
        succeeded: false,
        text: format!("Refused ({code}): {denial}"),
        summary: format!("{}{target}: refused ({code})", request.tool()),
        action_id: None,
        wrote: false,
    }
}

#[derive(Clone, Copy)]
pub struct Entry<'a> {
    pub context: &'a ActionContext,
    pub instance: &'a IntegrationInstance,
    pub request: &'a ActionRequest,
    pub secret: &'a str,
}

impl Entry<'_> {
    fn shown(&self, value: &Value) -> Value {
        scrub_value(redact(value), self.secret)
    }

    fn row(&self, outcome: Outcome, now: DateTime<Utc>) -> IntegrationAction {
        IntegrationAction {
            id: IntegrationActionId::new(),
            organization_id: self.instance.organization_id,
            run_id: self.context.run_id,
            card_id: self.context.card_id,
            instance_id: self.instance.id,
            subject: self.context.subject.id.clone(),
            actor: self.context.subject.actor.clone().unwrap_or_default(),
            tool: self.request.tool().to_owned(),
            access: self.request.access(),
            target: self.shown(&self.request.target()),
            arguments: self.shown(&self.request.arguments()),
            outcome,
            reason_code: None,
            reason: None,
            before: None,
            after: None,
            revertible: false,
            reverts_action_id: self.context.reverts_action_id,
            reverted_by_action_id: None,
            latency_ms: None,
            created_at: now,
            finished_at: None,
        }
    }

    fn reason_text(&self, reason: &str) -> String {
        scrub_text(reason, self.secret)
            .chars()
            .take(REASON_MAX_CHARS)
            .collect()
    }

    fn explain(&self, row: &mut IntegrationAction, outcome: Outcome, code: &str, reason: &str) {
        row.outcome = outcome;
        row.reason_code = Some(code.to_owned());
        row.reason = Some(self.reason_text(reason));
    }

    pub fn denied(&self, denial: &Denial, now: DateTime<Utc>) -> IntegrationAction {
        let mut row = self.row(Outcome::Denied, now);
        self.explain(
            &mut row,
            Outcome::Denied,
            denial.code(),
            &denial.to_string(),
        );
        row.finished_at = Some(now);
        row
    }

    pub fn read(
        &self,
        result: &Result<Value, ConnectorFailure>,
        latency_ms: i64,
        now: DateTime<Utc>,
    ) -> IntegrationAction {
        let mut row = self.row(Outcome::Succeeded, now);
        if let Err(failure) = result {
            self.explain(
                &mut row,
                Outcome::Failed,
                failure.code(),
                &failure.to_string(),
            );
        }
        row.latency_ms = Some(latency_ms);
        row.finished_at = Some(now);
        row
    }

    pub fn unreadable(
        &self,
        failure: &ConnectorFailure,
        latency_ms: i64,
        now: DateTime<Utc>,
    ) -> IntegrationAction {
        let mut row = self.row(Outcome::Failed, now);
        self.explain(
            &mut row,
            Outcome::Failed,
            BEFORE_UNREADABLE,
            &format!("the state before the write could not be read: {failure}"),
        );
        row.latency_ms = Some(latency_ms);
        row.finished_at = Some(now);
        row
    }

    pub fn pending(&self, before: Option<&Value>, now: DateTime<Utc>) -> IntegrationAction {
        let mut row = self.row(Outcome::Pending, now);
        row.before = before.map(|state| self.shown(state));
        row
    }

    pub fn complete(
        &self,
        mut pending: IntegrationAction,
        result: &Result<Value, ConnectorFailure>,
        after: Option<&Value>,
        latency_ms: i64,
        now: DateTime<Utc>,
    ) -> IntegrationAction {
        match result {
            Ok(_) => {
                pending.outcome = Outcome::Succeeded;
                pending.revertible = self.context.reverts_action_id.is_none()
                    && self
                        .request
                        .is_revertible(self.instance.server_version.as_deref());
            }
            Err(failure) => {
                self.explain(
                    &mut pending,
                    Outcome::Failed,
                    failure.code(),
                    &failure.to_string(),
                );
            }
        }
        pending.after = after.map(|state| self.shown(state));
        pending.latency_ms = Some(latency_ms);
        pending.finished_at = Some(now);
        pending
    }

    pub fn conclude(
        &self,
        action: &IntegrationAction,
        result: Option<&Value>,
        wrote: bool,
    ) -> ActionOutcome {
        let code = action.reason_code.as_deref().unwrap_or("unknown");
        let reason = action.reason.as_deref().unwrap_or_default();
        let (text, status) = match action.outcome {
            Outcome::Succeeded => (
                result
                    .map(|value| for_model(&scrub_value(value.clone(), self.secret)))
                    .unwrap_or_default(),
                "succeeded".to_owned(),
            ),
            Outcome::Denied => (
                format!("Refused ({code}): {reason}"),
                format!("refused ({code})"),
            ),
            Outcome::Failed | Outcome::Pending => (
                format!("Failed ({code}): {reason}"),
                format!("failed ({code})"),
            ),
        };
        let target = target_label(self.request)
            .map(|label| format!(" {label}"))
            .unwrap_or_default();
        ActionOutcome {
            succeeded: action.outcome == Outcome::Succeeded,
            text,
            summary: format!(
                "{}{target} on {}: {status}",
                self.request.tool(),
                self.instance.name.as_str()
            ),
            action_id: Some(action.id),
            wrote,
        }
    }
}

#[cfg(test)]
mod tests {
    use capitainerie_common::{ClientId, IntegrationInstanceId, OrganizationId};
    use chrono::Utc;
    use serde_json::{Value, json};

    use super::*;
    use crate::domain::{
        authorization::{Decision, Subject},
        integration::{
            BaseUrl, ConnectorFailure, Environment, InstanceName, IntegrationInstance,
            IntegrationKind,
            action::{Access, ActionCall, ActionRequest, N8nId, OdooModel},
            journal::{ActionContext, Denial, Grant, INTEGRATOR_AGENT, Outcome},
            redact::MASK,
        },
    };

    const SECRET: &str = "sk-test-123";

    fn instance(
        organization_id: OrganizationId,
        client_id: ClientId,
        kind: IntegrationKind,
        version: Option<&str>,
    ) -> IntegrationInstance {
        IntegrationInstance {
            id: IntegrationInstanceId::new(),
            organization_id,
            client_id,
            kind,
            name: InstanceName::try_from("Odoo prod".to_owned()).unwrap(),
            environment: Environment::Production,
            base_url: BaseUrl::try_from("https://odoo.acme.test".to_owned()).unwrap(),
            database: None,
            login: None,
            secret_preview: "…".into(),
            server_version: version.map(str::to_owned),
            checked_at: None,
            created_by: "user-1".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            archived_at: None,
        }
    }

    fn context(
        organization_id: OrganizationId,
        client_id: ClientId,
        grants: Vec<Grant>,
    ) -> ActionContext {
        ActionContext {
            subject: Subject {
                id: "user-1".into(),
                organization_id,
                actor: Some(INTEGRATOR_AGENT.into()),
                is_admin: false,
            },
            run_id: None,
            card_id: None,
            client_id,
            grants,
            reverts_action_id: None,
        }
    }

    fn grant(instance: &IntegrationInstance, access: Access) -> Grant {
        Grant {
            instance_id: instance.id,
            access,
        }
    }

    fn call(instance: &IntegrationInstance, request: ActionRequest) -> ActionCall {
        ActionCall {
            instance: instance.id,
            request,
        }
    }

    fn model(name: &str) -> OdooModel {
        OdooModel::try_from(name).unwrap()
    }

    fn write_partner(values: Value) -> ActionRequest {
        ActionRequest::OdooWrite {
            model: model("res.partner"),
            ids: vec![7],
            values: values.as_object().unwrap().clone(),
        }
    }

    fn read_partner() -> ActionRequest {
        ActionRequest::OdooCount {
            model: model("res.partner"),
            domain: vec![],
        }
    }

    fn update_workflow(publish: bool) -> ActionRequest {
        ActionRequest::N8nUpdateWorkflow {
            workflow_id: N8nId::try_from("wf1").unwrap(),
            name: None,
            nodes: vec![json!({"name": "Start"})],
            connections: Default::default(),
            settings: None,
            publish,
        }
    }

    struct World {
        organization_id: OrganizationId,
        client_id: ClientId,
    }

    impl World {
        fn new() -> Self {
            Self {
                organization_id: OrganizationId::new(),
                client_id: ClientId::new(),
            }
        }

        fn odoo(&self) -> IntegrationInstance {
            instance(
                self.organization_id,
                self.client_id,
                IntegrationKind::OdooJson2,
                Some("19.0"),
            )
        }

        fn n8n(&self, version: &str) -> IntegrationInstance {
            instance(
                self.organization_id,
                self.client_id,
                IntegrationKind::N8n,
                Some(version),
            )
        }

        fn context(&self, grants: Vec<Grant>) -> ActionContext {
            context(self.organization_id, self.client_id, grants)
        }
    }

    fn check(
        decision: &Decision,
        context: &ActionContext,
        instance: Option<&IntegrationInstance>,
        call: &ActionCall,
    ) -> Result<(), Denial> {
        ensure_allowed(decision, context, instance, call)
    }

    #[test]
    fn the_policy_decision_comes_before_everything_else() {
        let world = World::new();
        let odoo = world.odoo();
        let denied = Decision::Deny {
            reason: "no".into(),
        };

        let outcome = check(
            &denied,
            &world.context(vec![]),
            None,
            &call(&odoo, read_partner()),
        );

        assert_eq!(outcome, Err(Denial::PolicyDenied("no".into())));
    }

    #[test]
    fn a_missing_instance_or_one_of_another_organization_is_not_granted() {
        let world = World::new();
        let odoo = world.odoo();
        let foreign = instance(
            OrganizationId::new(),
            world.client_id,
            IntegrationKind::OdooJson2,
            None,
        );
        let context = world.context(vec![grant(&odoo, Access::Write)]);

        assert_eq!(
            check(
                &Decision::Allow,
                &context,
                None,
                &call(&odoo, read_partner())
            ),
            Err(Denial::InstanceNotGranted)
        );
        assert_eq!(
            check(
                &Decision::Allow,
                &context,
                Some(&foreign),
                &call(&foreign, read_partner())
            ),
            Err(Denial::InstanceNotGranted)
        );
    }

    #[test]
    fn an_archived_instance_is_refused_before_its_client_and_grants_are_looked_at() {
        let world = World::new();
        let mut odoo = world.odoo();
        odoo.archived_at = Some(Utc::now());
        odoo.client_id = ClientId::new();

        let outcome = check(
            &Decision::Allow,
            &world.context(vec![]),
            Some(&odoo),
            &call(&odoo, read_partner()),
        );

        assert_eq!(outcome, Err(Denial::InstanceArchived));
    }

    #[test]
    fn an_instance_of_another_client_is_refused_before_the_grants() {
        let world = World::new();
        let mut odoo = world.odoo();
        odoo.client_id = ClientId::new();

        let outcome = check(
            &Decision::Allow,
            &world.context(vec![]),
            Some(&odoo),
            &call(&odoo, read_partner()),
        );

        assert_eq!(outcome, Err(Denial::InstanceOfAnotherClient));
    }

    #[test]
    fn an_ungranted_instance_and_a_read_grant_on_a_write_are_told_apart() {
        let world = World::new();
        let odoo = world.odoo();
        let other = world.odoo();

        let ungranted = check(
            &Decision::Allow,
            &world.context(vec![grant(&other, Access::Write)]),
            Some(&odoo),
            &call(&odoo, read_partner()),
        );
        let read_only = check(
            &Decision::Allow,
            &world.context(vec![grant(&odoo, Access::Read)]),
            Some(&odoo),
            &call(&odoo, write_partner(json!({"name": "Acme"}))),
        );
        let read = check(
            &Decision::Allow,
            &world.context(vec![grant(&odoo, Access::Read)]),
            Some(&odoo),
            &call(&odoo, read_partner()),
        );

        assert_eq!(ungranted, Err(Denial::InstanceNotGranted));
        assert_eq!(read_only, Err(Denial::WriteNotGranted));
        assert_eq!(read, Ok(()));
    }

    #[test]
    fn a_tool_of_the_other_family_is_refused_before_the_sensitive_model_check() {
        let world = World::new();
        let n8n = world.n8n("2");
        let sensitive = ActionRequest::OdooRead {
            model: model("ir.config_parameter"),
            ids: vec![1],
            fields: vec![],
        };

        let outcome = check(
            &Decision::Allow,
            &world.context(vec![grant(&n8n, Access::Write)]),
            Some(&n8n),
            &call(&n8n, sensitive),
        );

        assert_eq!(outcome, Err(Denial::ToolNotForInstance));
    }

    #[test]
    fn a_sensitive_model_is_refused_even_with_a_write_grant() {
        let world = World::new();
        let odoo = world.odoo();
        let context = world.context(vec![grant(&odoo, Access::Write)]);
        let sensitive = ActionRequest::OdooSearchRead {
            model: model("res.users.apikeys"),
            domain: vec![],
            fields: vec![],
            limit: 10,
            offset: 0,
            order: None,
        };

        let outcome = check(
            &Decision::Allow,
            &context,
            Some(&odoo),
            &call(&odoo, sensitive),
        );
        let listing = check(
            &Decision::Allow,
            &context,
            Some(&odoo),
            &call(
                &odoo,
                ActionRequest::OdooListModels {
                    query: None,
                    limit: 10,
                },
            ),
        );

        assert_eq!(
            outcome,
            Err(Denial::SensitiveModel("res.users.apikeys".into()))
        );
        assert_eq!(listing, Ok(()));
    }

    #[test]
    fn n8n_1x_refuses_to_change_a_published_workflow_without_publish() {
        let world = World::new();
        let one = world.n8n("1");
        let two = world.n8n("2");
        let published = json!({"id": "wf1", "name": "Relance", "active": true});
        let draft = json!({"id": "wf1", "name": "Relance", "active": false});
        let refusal = Denial::NotSupported(
            "n8n 1.x would publish this change at once: unpublish the workflow first or send publish: true"
                .into(),
        );

        assert_eq!(
            publication_guard(&one, &update_workflow(false), Some(&published)),
            Err(refusal)
        );
        assert_eq!(
            publication_guard(&one, &update_workflow(true), Some(&published)),
            Ok(())
        );
        assert_eq!(
            publication_guard(&one, &update_workflow(false), Some(&draft)),
            Ok(())
        );
        assert_eq!(
            publication_guard(&one, &update_workflow(false), None),
            Ok(())
        );
        assert_eq!(
            publication_guard(&two, &update_workflow(false), Some(&published)),
            Ok(())
        );
        assert_eq!(
            publication_guard(&one, &read_partner(), Some(&published)),
            Ok(())
        );
    }

    #[test]
    fn an_update_without_name_or_settings_takes_them_from_the_copy_before() {
        let before = json!({
            "id": "wf1",
            "name": "Relance factures",
            "settings": {"executionOrder": "v1"}
        });
        let mut request = update_workflow(false);

        fill_from_before(&mut request, Some(&before));

        let ActionRequest::N8nUpdateWorkflow { name, settings, .. } = request else {
            unreachable!()
        };
        assert_eq!(name.as_deref(), Some("Relance factures"));
        assert_eq!(
            Value::Object(settings.unwrap()),
            json!({"executionOrder": "v1"})
        );
    }

    #[test]
    fn a_given_name_and_settings_are_kept_and_missing_settings_become_empty() {
        let before = json!({"name": "Old", "settings": {"a": 1}});
        let mut given = update_workflow(false);
        if let ActionRequest::N8nUpdateWorkflow { name, settings, .. } = &mut given {
            *name = Some("New".into());
            *settings = Some(json!({"b": 2}).as_object().unwrap().clone());
        }
        let mut bare = update_workflow(false);

        fill_from_before(&mut given, Some(&before));
        fill_from_before(&mut bare, Some(&json!({"name": "Old"})));

        let ActionRequest::N8nUpdateWorkflow { name, settings, .. } = given else {
            unreachable!()
        };
        assert_eq!(name.as_deref(), Some("New"));
        assert_eq!(Value::Object(settings.unwrap()), json!({"b": 2}));
        let ActionRequest::N8nUpdateWorkflow { name, settings, .. } = bare else {
            unreachable!()
        };
        assert_eq!(name.as_deref(), Some("Old"));
        assert_eq!(Value::Object(settings.unwrap()), json!({}));
    }

    #[test]
    fn filling_leaves_other_requests_alone() {
        let mut request = write_partner(json!({"name": "Acme"}));
        let expected = request.clone();

        fill_from_before(&mut request, Some(&json!({"name": "x"})));

        assert_eq!(request, expected);
    }

    fn entry<'a>(
        context: &'a ActionContext,
        instance: &'a IntegrationInstance,
        request: &'a ActionRequest,
    ) -> Entry<'a> {
        Entry {
            context,
            instance,
            request,
            secret: SECRET,
        }
    }

    #[test]
    fn a_denied_row_carries_its_code_and_reason_and_is_already_finished() {
        let world = World::new();
        let odoo = world.odoo();
        let context = world.context(vec![grant(&odoo, Access::Read)]);
        let request = write_partner(json!({"name": "Acme"}));
        let now = Utc::now();

        let row = entry(&context, &odoo, &request).denied(&Denial::WriteNotGranted, now);

        assert_eq!(row.outcome, Outcome::Denied);
        assert_eq!(row.reason_code.as_deref(), Some("write_not_granted"));
        assert_eq!(
            row.reason.as_deref(),
            Some("this instance is granted read-only to the intervention")
        );
        assert_eq!(row.tool, "odoo_write");
        assert_eq!(row.access, Access::Write);
        assert_eq!(row.subject, "user-1");
        assert_eq!(row.actor, INTEGRATOR_AGENT);
        assert_eq!(row.instance_id, odoo.id);
        assert_eq!(row.organization_id, world.organization_id);
        assert_eq!(row.finished_at, Some(now));
        assert!(!row.revertible);
        assert!(row.before.is_none() && row.after.is_none());
    }

    #[test]
    fn a_read_row_is_finished_and_names_the_failure_code() {
        let world = World::new();
        let odoo = world.odoo();
        let context = world.context(vec![grant(&odoo, Access::Read)]);
        let request = read_partner();

        let done = entry(&context, &odoo, &request).read(&Ok(json!(3)), 12, Utc::now());
        let failed = entry(&context, &odoo, &request).read(
            &Err(ConnectorFailure::Timeout),
            30_000,
            Utc::now(),
        );
        let blocked = entry(&context, &odoo, &request).read(
            &Err(ConnectorFailure::Blocked("10.0.0.5".into())),
            1,
            Utc::now(),
        );

        assert_eq!(done.outcome, Outcome::Succeeded);
        assert_eq!(done.latency_ms, Some(12));
        assert!(done.finished_at.is_some());
        assert!(done.reason_code.is_none());
        assert!(!done.revertible);
        assert_eq!(failed.outcome, Outcome::Failed);
        assert_eq!(failed.reason_code.as_deref(), Some("connection_failed"));
        assert_eq!(
            failed.reason.as_deref(),
            Some("the instance did not answer in time")
        );
        assert_eq!(blocked.reason_code.as_deref(), Some("network_blocked"));
    }

    #[test]
    fn a_pending_write_row_keeps_the_redacted_copy_before_and_is_not_finished() {
        let world = World::new();
        let odoo = world.odoo();
        let context = world.context(vec![grant(&odoo, Access::Write)]);
        let request = write_partner(json!({"api_key": "k-new", "name": "Acme"}));
        let before = json!([{"id": 7, "api_key": "k-old", "name": "Old"}]);

        let row = entry(&context, &odoo, &request).pending(Some(&before), Utc::now());

        assert_eq!(row.outcome, Outcome::Pending);
        assert!(row.finished_at.is_none());
        assert_eq!(row.before.as_ref().unwrap()[0]["api_key"], MASK);
        assert_eq!(row.before.as_ref().unwrap()[0]["name"], "Old");
        assert_eq!(row.arguments["values"]["api_key"], MASK);
        assert_eq!(row.arguments["values"]["name"], "Acme");
        let stored = serde_json::to_string(&(&row.arguments, &row.before, &row.target)).unwrap();
        assert!(!stored.contains("k-new") && !stored.contains("k-old"));
    }

    #[test]
    fn a_completed_write_is_revertible_only_when_it_succeeded_and_the_action_allows_it() {
        let world = World::new();
        let odoo = world.odoo();
        let context = world.context(vec![grant(&odoo, Access::Write)]);
        let request = write_partner(json!({"name": "Acme"}));
        let entry = entry(&context, &odoo, &request);
        let after = json!([{"id": 7, "name": "Acme", "api_key": "k-after"}]);

        let succeeded = entry.complete(
            entry.pending(None, Utc::now()),
            &Ok(json!(true)),
            Some(&after),
            40,
            Utc::now(),
        );
        let failed = entry.complete(
            entry.pending(None, Utc::now()),
            &Err(ConnectorFailure::Remote {
                status: 500,
                message: "boom".into(),
            }),
            None,
            40,
            Utc::now(),
        );

        assert_eq!(succeeded.outcome, Outcome::Succeeded);
        assert!(succeeded.revertible);
        assert_eq!(succeeded.latency_ms, Some(40));
        assert!(succeeded.finished_at.is_some());
        assert_eq!(succeeded.after.as_ref().unwrap()[0]["api_key"], MASK);
        assert!(!succeeded.after.unwrap().to_string().contains("k-after"));
        assert_eq!(failed.outcome, Outcome::Failed);
        assert!(!failed.revertible);
        assert_eq!(failed.reason_code.as_deref(), Some("connection_failed"));
        assert_eq!(
            failed.reason.as_deref(),
            Some("the instance answered 500: boom")
        );
        assert!(failed.after.is_none());
    }

    #[test]
    fn creating_a_workflow_is_revertible_on_n8n_2_only() {
        let world = World::new();
        let request = ActionRequest::N8nCreateWorkflow {
            name: "New".into(),
            nodes: vec![],
            connections: Default::default(),
            settings: Default::default(),
        };
        let revertible = |version: &str| {
            let n8n = world.n8n(version);
            let context = world.context(vec![grant(&n8n, Access::Write)]);
            let entry = entry(&context, &n8n, &request);
            entry
                .complete(
                    entry.pending(None, Utc::now()),
                    &Ok(json!({"id": "wf9"})),
                    None,
                    5,
                    Utc::now(),
                )
                .revertible
        };

        assert!(revertible("2"));
        assert!(!revertible("1"));
    }

    #[test]
    fn a_failure_while_reading_the_state_before_is_a_failed_row_without_a_write() {
        let world = World::new();
        let odoo = world.odoo();
        let context = world.context(vec![grant(&odoo, Access::Write)]);
        let request = write_partner(json!({"name": "Acme"}));

        let row = entry(&context, &odoo, &request).unreadable(
            &ConnectorFailure::Unreachable("dns".into()),
            9,
            Utc::now(),
        );

        assert_eq!(row.outcome, Outcome::Failed);
        assert_eq!(row.reason_code.as_deref(), Some(BEFORE_UNREADABLE));
        assert_eq!(
            row.reason.as_deref(),
            Some(
                "the state before the write could not be read: the instance cannot be reached: dns"
            )
        );
        assert!(!row.revertible);
        assert!(row.finished_at.is_some());
    }

    #[test]
    fn the_instance_secret_and_oversized_reasons_never_reach_the_row() {
        let world = World::new();
        let odoo = world.odoo();
        let context = world.context(vec![grant(&odoo, Access::Read)]);
        let request = read_partner();
        let echoed = ConnectorFailure::Remote {
            status: 401,
            message: format!("bad key {SECRET} {}", "x".repeat(5000)),
        };

        let row = entry(&context, &odoo, &request).read(&Err(echoed), 3, Utc::now());

        let reason = row.reason.unwrap();
        assert!(!reason.contains(SECRET));
        assert!(reason.contains(MASK));
        assert_eq!(reason.chars().count(), REASON_MAX_CHARS);
    }

    #[test]
    fn the_instance_secret_is_scrubbed_from_the_shown_states_and_the_model_text() {
        let world = World::new();
        let odoo = world.odoo();
        let context = world.context(vec![grant(&odoo, Access::Write)]);
        let request = write_partner(json!({"name": "Acme"}));
        let entry = entry(&context, &odoo, &request);
        let leaky = json!([{"note": format!("the key is {SECRET}")}]);

        let row = entry.complete(
            entry.pending(Some(&leaky), Utc::now()),
            &Ok(leaky.clone()),
            Some(&leaky),
            1,
            Utc::now(),
        );
        let outcome = entry.conclude(&row, Some(&leaky), true);

        assert!(!row.before.unwrap().to_string().contains(SECRET));
        assert!(!row.after.unwrap().to_string().contains(SECRET));
        assert!(!outcome.text.contains(SECRET));
    }

    #[test]
    fn the_model_sees_the_masked_result_on_success_a_refusal_or_a_failure() {
        let world = World::new();
        let odoo = world.odoo();
        let context = world.context(vec![grant(&odoo, Access::Write)]);
        let request = write_partner(json!({"name": "Acme"}));
        let entry = entry(&context, &odoo, &request);
        let result = json!([{"id": 7, "api_key": "k-live"}]);

        let succeeded = entry.complete(
            entry.pending(None, Utc::now()),
            &Ok(result.clone()),
            None,
            2,
            Utc::now(),
        );
        let refused = entry.denied(&Denial::WriteNotGranted, Utc::now());
        let failed = entry.complete(
            entry.pending(None, Utc::now()),
            &Err(ConnectorFailure::Timeout),
            None,
            2,
            Utc::now(),
        );

        let ok = entry.conclude(&succeeded, Some(&result), true);
        assert!(ok.succeeded && ok.wrote);
        assert_eq!(ok.action_id, Some(succeeded.id));
        assert_eq!(
            ok.text,
            crate::domain::integration::redact::for_model(&result)
        );
        assert!(!ok.text.contains("k-live"));
        assert_eq!(ok.summary, "odoo_write res.partner on Odoo prod: succeeded");

        let no = entry.conclude(&refused, None, false);
        assert!(!no.succeeded && !no.wrote);
        assert_eq!(
            no.text,
            "Refused (write_not_granted): this instance is granted read-only to the intervention"
        );
        assert_eq!(
            no.summary,
            "odoo_write res.partner on Odoo prod: refused (write_not_granted)"
        );

        let bad = entry.conclude(&failed, None, true);
        assert!(!bad.succeeded && bad.wrote);
        assert_eq!(
            bad.text,
            "Failed (connection_failed): the instance did not answer in time"
        );
        assert_eq!(
            bad.summary,
            "odoo_write res.partner on Odoo prod: failed (connection_failed)"
        );
    }

    #[test]
    fn the_summary_names_the_n8n_workflow() {
        let world = World::new();
        let n8n = world.n8n("2");
        let context = world.context(vec![grant(&n8n, Access::Write)]);
        let request = update_workflow(true);
        let entry = entry(&context, &n8n, &request);
        let row = entry.read(&Ok(json!({})), 1, Utc::now());

        assert_eq!(
            entry.conclude(&row, Some(&json!({})), false).summary,
            "n8n_update_workflow wf1 on Odoo prod: succeeded"
        );
    }

    #[test]
    fn a_long_result_is_cut_for_the_model() {
        let world = World::new();
        let odoo = world.odoo();
        let context = world.context(vec![grant(&odoo, Access::Read)]);
        let request = read_partner();
        let entry = entry(&context, &odoo, &request);
        let huge = json!("a".repeat(30_000));
        let row = entry.read(&Ok(huge.clone()), 1, Utc::now());

        let outcome = entry.conclude(&row, Some(&huge), false);

        assert!(outcome.text.contains("[truncated:"));
        assert!(outcome.text.chars().count() < 24_100);
    }

    #[test]
    fn a_refusal_without_a_row_still_tells_the_model_why() {
        let outcome = unrecorded(&Denial::InstanceNotGranted, &read_partner());

        assert!(!outcome.succeeded && !outcome.wrote);
        assert!(outcome.action_id.is_none());
        assert_eq!(
            outcome.text,
            "Refused (instance_not_granted): this instance is not granted to the intervention"
        );
        assert_eq!(
            outcome.summary,
            "odoo_count res.partner: refused (instance_not_granted)"
        );
    }
}
