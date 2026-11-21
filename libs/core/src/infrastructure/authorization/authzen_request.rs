use serde_json::{Map, Value, json};

use crate::domain::{
    agent_task::is_agent_actor,
    authorization::{AccessRequest, Action, ResourceKind},
    conversation::CAPTAIN_AGENT,
    integration::journal::INTEGRATOR_AGENT,
};

const NAMESPACE: &str = "Capitainerie";
const NO_RESOURCE_ID: &str = "none";

pub fn evaluation_of(request: &AccessRequest) -> Value {
    json!({
        "subject": {
            "type": "user",
            "id": request.subject.id,
        },
        "action": {
            "name": request.action.as_str(),
        },
        "resource": {
            "type": format!("{NAMESPACE}::{}", entity_type(request.resource.kind)),
            "id": request
                .resource
                .id
                .map_or_else(|| NO_RESOURCE_ID.to_owned(), |id| id.to_string()),
            "properties": {
                "organization": request.resource.organization_id.to_string(),
            },
        },
        "context": context_of(request),
    })
}

pub fn actor_kind(actor: Option<&str>) -> &'static str {
    match actor {
        None => "human",
        Some(actor) if actor == CAPTAIN_AGENT => "captain",
        Some(actor) if actor == INTEGRATOR_AGENT => "integrator",
        Some(actor) if is_agent_actor(actor) => "registry",
        Some(_) => "agent",
    }
}

fn context_of(request: &AccessRequest) -> Value {
    let subject = &request.subject;
    let mut context = Map::new();
    context.insert(
        "organization".into(),
        json!(subject.organization_id.to_string()),
    );
    context.insert("is_admin".into(), json!(subject.is_admin));
    context.insert(
        "actor_kind".into(),
        json!(actor_kind(subject.actor.as_deref())),
    );
    if let Some(actor) = &subject.actor {
        context.insert("actor".into(), json!(actor));
    }
    match request.action {
        Action::CardUpdate { column } => {
            context.insert("intent".into(), json!("update"));
            context.insert("column".into(), json!(column.as_str()));
        }
        Action::CardLink => {
            context.insert("intent".into(), json!("link"));
        }
        Action::CardEdit => {
            context.insert("intent".into(), json!("edit"));
        }
        Action::CardMove { from, to } => {
            context.insert("from".into(), json!(from.as_str()));
            context.insert("to".into(), json!(to.as_str()));
        }
        Action::IntegrationCall { access } => {
            context.insert("access".into(), json!(access.as_str()));
        }
        Action::McpCall { initiated, granted } => {
            context.insert("initiated".into(), json!(initiated));
            context.insert("granted".into(), json!(granted));
        }
        Action::TranscriptCreate
        | Action::TranscriptionCreate
        | Action::DocumentRead
        | Action::RunRead
        | Action::CardCreate
        | Action::CardRead
        | Action::CardComment
        | Action::ProviderRead
        | Action::ProviderManage
        | Action::UserProviderManage
        | Action::ModelsAssign
        | Action::CardExecute
        | Action::GithubManage
        | Action::ClientRead
        | Action::ClientManage
        | Action::MemberRead
        | Action::GithubRead
        | Action::ConversationRead
        | Action::ConversationWrite
        | Action::IntegrationRead
        | Action::IntegrationManage
        | Action::CardIntervene
        | Action::IntegrationRevert
        | Action::AgentRead
        | Action::AgentManage
        | Action::McpConnectorManage
        | Action::McpConnect
        | Action::AgentRun
        | Action::SkillRead
        | Action::SkillManage
        | Action::ApprovalRead
        | Action::ApprovalDecide
        | Action::SourceRead
        | Action::SourceManage
        | Action::RoutineRead
        | Action::RoutineManage
        | Action::AuditRead
        | Action::BudgetManage
        | Action::InboxRead
        | Action::ArtifactRead => {}
    }
    Value::Object(context)
}

fn entity_type(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Transcript => "Transcript",
        ResourceKind::Transcription => "Transcription",
        ResourceKind::Document => "Document",
        ResourceKind::Run => "Run",
        ResourceKind::Card => "Card",
        ResourceKind::Provider => "Provider",
        ResourceKind::UserProvider => "UserProvider",
        ResourceKind::ModelAssignments => "ModelAssignments",
        ResourceKind::GithubInstallation => "GithubInstallation",
        ResourceKind::GithubApp => "GithubApp",
        ResourceKind::Client => "Client",
        ResourceKind::Project => "Project",
        ResourceKind::Member => "Member",
        ResourceKind::Conversation => "Conversation",
        ResourceKind::IntegrationInstance => "IntegrationInstance",
        ResourceKind::Agent => "Agent",
        ResourceKind::McpConnector => "McpConnector",
        ResourceKind::Skill => "Skill",
        ResourceKind::Approval => "Approval",
        ResourceKind::Source => "Source",
        ResourceKind::Routine => "Routine",
        ResourceKind::Audit => "Audit",
        ResourceKind::Budget => "Budget",
        ResourceKind::Inbox => "Inbox",
        ResourceKind::Artifact => "Artifact",
    }
}
