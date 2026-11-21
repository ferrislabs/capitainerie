#![allow(clippy::unwrap_used)]

use std::{collections::HashSet, fs, path::PathBuf, str::FromStr};

use capitainerie_common::OrganizationId;
use capitainerie_core::domain::{
    self,
    authorization::{AccessRequest, Action, Decision, ResourceKind, Subject, ports::Authorizer},
    card::Column,
    conversation::CAPTAIN_AGENT,
    execution::EXECUTOR_AGENT,
    integration::{action::Access, journal::INTEGRATOR_AGENT},
};
use capitainerie_core::infrastructure::authorization::{
    LocalPolicyEngine, actor_kind, evaluation_of,
};
use cedar_policy::{
    Authorizer as CedarAuthorizer, Context, Decision as CedarDecision, Entities, Entity, EntityId,
    EntityTypeName, EntityUid, PolicySet, Request, Schema, SchemaFragment, ValidationMode,
    Validator,
};
use serde_json::{Value, json};

const BUILTIN_SCHEMA: &str = include_str!("fixtures/ferriskey-builtin-1078670.cedarschema");
const REALM_SCHEMA: &str = include_str!("../../../deploy/ferriskey/authz/schema.cedarschema");
const REALM_NAMESPACE: &str = "Capitainerie";
const ACTION_COUNT: usize = 46;

const RESOURCE_KINDS: [ResourceKind; 25] = [
    ResourceKind::Transcript,
    ResourceKind::Transcription,
    ResourceKind::Document,
    ResourceKind::Run,
    ResourceKind::Card,
    ResourceKind::Provider,
    ResourceKind::UserProvider,
    ResourceKind::ModelAssignments,
    ResourceKind::GithubInstallation,
    ResourceKind::GithubApp,
    ResourceKind::Client,
    ResourceKind::Project,
    ResourceKind::Member,
    ResourceKind::Conversation,
    ResourceKind::IntegrationInstance,
    ResourceKind::Agent,
    ResourceKind::McpConnector,
    ResourceKind::Skill,
    ResourceKind::Approval,
    ResourceKind::Source,
    ResourceKind::Routine,
    ResourceKind::Audit,
    ResourceKind::Budget,
    ResourceKind::Inbox,
    ResourceKind::Artifact,
];

const ACCESSES: [Access; 2] = [Access::Read, Access::Write];

fn actors() -> [Option<String>; 5] {
    [
        None,
        Some(CAPTAIN_AGENT.to_owned()),
        Some(INTEGRATOR_AGENT.to_owned()),
        Some(domain::agent_task::actor_of("darnold")),
        Some("agent-writer".to_owned()),
    ]
}

fn ordinal(action: Action) -> usize {
    match action {
        Action::TranscriptCreate => 0,
        Action::TranscriptionCreate => 1,
        Action::DocumentRead => 2,
        Action::RunRead => 3,
        Action::CardCreate => 4,
        Action::CardRead => 5,
        Action::CardUpdate { .. } => 6,
        Action::CardLink => 7,
        Action::CardEdit => 8,
        Action::CardComment => 9,
        Action::CardMove { .. } => 10,
        Action::ProviderRead => 11,
        Action::ProviderManage => 12,
        Action::UserProviderManage => 13,
        Action::ModelsAssign => 14,
        Action::CardExecute => 15,
        Action::GithubManage => 16,
        Action::ClientRead => 17,
        Action::ClientManage => 18,
        Action::MemberRead => 19,
        Action::GithubRead => 20,
        Action::ConversationRead => 21,
        Action::ConversationWrite => 22,
        Action::IntegrationRead => 23,
        Action::IntegrationManage => 24,
        Action::IntegrationCall { .. } => 25,
        Action::CardIntervene => 26,
        Action::IntegrationRevert => 27,
        Action::AgentRead => 28,
        Action::AgentManage => 29,
        Action::McpConnectorManage => 30,
        Action::McpConnect => 31,
        Action::AgentRun => 32,
        Action::McpCall { .. } => 33,
        Action::SkillRead => 34,
        Action::SkillManage => 35,
        Action::ApprovalRead => 36,
        Action::ApprovalDecide => 37,
        Action::SourceRead => 38,
        Action::SourceManage => 39,
        Action::RoutineRead => 40,
        Action::RoutineManage => 41,
        Action::AuditRead => 42,
        Action::BudgetManage => 43,
        Action::InboxRead => 44,
        Action::ArtifactRead => 45,
    }
}

fn every_action() -> Vec<Action> {
    let mut actions = vec![
        Action::TranscriptCreate,
        Action::TranscriptionCreate,
        Action::DocumentRead,
        Action::RunRead,
        Action::CardCreate,
        Action::CardRead,
        Action::CardLink,
        Action::CardEdit,
        Action::CardComment,
        Action::ProviderRead,
        Action::ProviderManage,
        Action::UserProviderManage,
        Action::ModelsAssign,
        Action::CardExecute,
        Action::GithubManage,
        Action::ClientRead,
        Action::ClientManage,
        Action::MemberRead,
        Action::GithubRead,
        Action::ConversationRead,
        Action::ConversationWrite,
        Action::IntegrationRead,
        Action::IntegrationManage,
        Action::CardIntervene,
        Action::IntegrationRevert,
        Action::AgentRead,
        Action::AgentManage,
        Action::McpConnectorManage,
        Action::McpConnect,
        Action::AgentRun,
        Action::SkillRead,
        Action::SkillManage,
        Action::ApprovalRead,
        Action::ApprovalDecide,
        Action::SourceRead,
        Action::SourceManage,
        Action::RoutineRead,
        Action::RoutineManage,
        Action::AuditRead,
        Action::BudgetManage,
        Action::InboxRead,
        Action::ArtifactRead,
    ];
    for column in Column::ALL {
        actions.push(Action::CardUpdate { column });
        for to in Column::ALL {
            actions.push(Action::CardMove { from: column, to });
        }
    }
    for access in ACCESSES {
        actions.push(Action::IntegrationCall { access });
    }
    for initiated in [true, false] {
        for granted in [true, false] {
            actions.push(Action::McpCall { initiated, granted });
        }
    }
    actions
}

fn schema() -> Schema {
    let fragment = |source: &str| SchemaFragment::from_cedarschema_str(source).unwrap().0;
    Schema::from_schema_fragments([fragment(BUILTIN_SCHEMA), fragment(REALM_SCHEMA)]).unwrap()
}

fn policy_sources() -> Vec<String> {
    let directory =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../deploy/ferriskey/authz/policies");
    let mut files: Vec<PathBuf> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "cedar"))
        .collect();
    files.sort();
    files
        .iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect()
}

fn policies() -> PolicySet {
    PolicySet::from_str(&policy_sources().join("\n")).unwrap()
}

fn uid(ty: &str, id: &str) -> EntityUid {
    EntityUid::from_type_name_and_id(EntityTypeName::from_str(ty).unwrap(), EntityId::new(id))
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap()
}

fn cedar_allows(body: &Value, schema: &Schema, policies: &PolicySet) -> Result<bool, String> {
    let subject = &body["subject"];
    let principal = uid("FerrisKey::User", text(subject, "id"));
    let user = Entity::from_json_value(
        json!({
            "uid": {"type": "FerrisKey::User", "id": text(subject, "id")},
            "attrs": {
                "username": text(subject, "id"),
                "email_verified": false,
                "enabled": true,
                "roles": [],
            },
            "parents": [],
        }),
        Some(schema),
    )
    .map_err(|e| e.to_string())?;

    let resource_ref = &body["resource"];
    let resource = uid(text(resource_ref, "type"), text(resource_ref, "id"));
    let resource_entity = Entity::from_json_value(
        json!({
            "uid": {"type": text(resource_ref, "type"), "id": text(resource_ref, "id")},
            "attrs": resource_ref["properties"],
            "parents": [],
        }),
        Some(schema),
    )
    .map_err(|e| e.to_string())?;

    let entities = Entities::from_entities([user, resource_entity], Some(schema))
        .map_err(|e| e.to_string())?;
    let action = uid(
        &format!("{REALM_NAMESPACE}::Action"),
        text(&body["action"], "name"),
    );
    let context = Context::from_json_value(body["context"].clone(), Some((schema, &action)))
        .map_err(|e| e.to_string())?;
    let request = Request::new(principal, action, resource, context, Some(schema))
        .map_err(|e| e.to_string())?;

    let response = CedarAuthorizer::new().is_authorized(&request, policies, &entities);
    if response.diagnostics().errors().next().is_some() {
        return Err(format!(
            "policy evaluation errors: {:?}",
            response.diagnostics()
        ));
    }
    Ok(response.decision() == CedarDecision::Allow)
}

fn request_of(
    action: Action,
    kind: ResourceKind,
    actor: Option<&String>,
    is_admin: bool,
    same_organization: bool,
) -> AccessRequest {
    let mut request = AccessRequest::new(
        Subject {
            id: "0197c6a0-0000-7000-8000-000000000001".into(),
            organization_id: OrganizationId::new(),
            actor: actor.cloned(),
            is_admin,
        },
        action,
        kind,
        None,
    );
    if !same_organization {
        request.resource.organization_id = OrganizationId::new();
    }
    request
}

#[test]
fn the_policies_validate_against_the_schema_in_strict_mode() {
    let result = Validator::new(schema()).validate(&policies(), ValidationMode::Strict);

    assert!(
        result.validation_passed(),
        "{:?}",
        result.validation_errors().collect::<Vec<_>>()
    );
    assert_eq!(result.validation_warnings().count(), 0);
}

#[test]
fn every_policy_carries_a_distinct_meaningful_id() {
    let set = policies();
    let mut seen = HashSet::new();

    for policy in set.policies() {
        let id = policy.annotation("id").expect("every policy has an @id");
        assert!(seen.insert(id.to_owned()), "duplicate {id}");
    }
    assert!(!seen.is_empty());
}

#[test]
fn the_enumeration_covers_every_action_variant() {
    let covered: HashSet<usize> = every_action().into_iter().map(ordinal).collect();

    assert_eq!(covered.len(), ACTION_COUNT);
    assert!(covered.iter().all(|ordinal| *ordinal < ACTION_COUNT));
}

#[test]
fn the_actor_kind_follows_the_local_classification() {
    assert_eq!(actor_kind(None), "human");
    assert_eq!(actor_kind(Some(CAPTAIN_AGENT)), "captain");
    assert_eq!(actor_kind(Some(INTEGRATOR_AGENT)), "integrator");
    assert_eq!(actor_kind(Some("agent:darnold")), "registry");
    assert_eq!(actor_kind(Some(EXECUTOR_AGENT)), "agent");
    assert_eq!(actor_kind(Some("agent-writer")), "agent");
}

#[test]
fn the_evaluation_body_is_authzen_shaped() {
    let request = request_of(
        Action::CardMove {
            from: Column::Todo,
            to: Column::Done,
        },
        ResourceKind::Card,
        Some(&"agent-writer".to_owned()),
        false,
        true,
    );

    let body = evaluation_of(&request);

    assert_eq!(body["subject"]["type"], "user");
    assert_eq!(body["subject"]["id"], request.subject.id);
    assert_eq!(body["action"]["name"], "card.move");
    assert_eq!(body["resource"]["type"], "Capitainerie::Card");
    assert_eq!(
        body["resource"]["properties"]["organization"],
        request.resource.organization_id.to_string()
    );
    assert_eq!(body["context"]["from"], "todo");
    assert_eq!(body["context"]["to"], "done");
    assert_eq!(body["context"]["actor"], "agent-writer");
    assert_eq!(body["context"]["actor_kind"], "agent");
    assert_eq!(body["context"]["is_admin"], false);
}

#[tokio::test]
async fn cedar_decides_exactly_like_the_local_engine() {
    let schema = schema();
    let policies = policies();
    let mut cases = 0usize;
    let mut failures = Vec::new();

    for action in every_action() {
        for kind in RESOURCE_KINDS {
            for actor in actors() {
                for is_admin in [true, false] {
                    for same_organization in [true, false] {
                        let request =
                            request_of(action, kind, actor.as_ref(), is_admin, same_organization);
                        let local =
                            LocalPolicyEngine.evaluate(&request).await.unwrap() == Decision::Allow;
                        let body = evaluation_of(&request);
                        cases += 1;
                        match cedar_allows(&body, &schema, &policies) {
                            Ok(cedar) if cedar == local => {}
                            Ok(cedar) => failures.push(format!(
                                "local={local} cedar={cedar}\n{request:#?}\n{body:#}"
                            )),
                            Err(error) => failures.push(format!("{error}\n{request:#?}\n{body:#}")),
                        }
                    }
                }
            }
        }
    }

    eprintln!("enumerated {cases} cases, {} mismatches", failures.len());
    assert!(
        failures.is_empty(),
        "{} of {cases} cases differ, first:\n{}",
        failures.len(),
        failures
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n---\n")
    );
}
