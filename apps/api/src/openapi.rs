use capitainerie_handlers::{errors::ApiErrorResponse, health};
use capitainerie_handlers_analytics::{
    self as analytics,
    agents::AgentResponse,
    costs::{CostDimensionParam, CostRowResponse, CostTotals, CostsResponse},
};
use capitainerie_handlers_approvals::{
    self as approvals,
    decide_approval::{ApprovalDecisionDto, DecideApprovalRequest},
    response::{
        ApprovalKindDto, ApprovalListResponse, ApprovalResponse, ApprovalStatusDto,
        DecideApprovalResponse,
    },
};
use capitainerie_handlers_artifacts as artifacts;
use capitainerie_handlers_audit as audit;
use capitainerie_handlers_cards::{
    self as cards,
    comment_card::CommentCardRequest,
    create_card::CreateCardRequest,
    execute_card::{ExecuteCardRequest, ExecuteCardResponse},
    intervene_card::{
        GrantAccessDto, InstanceGrantRequest, InterveneCardRequest, InterveneCardResponse,
    },
    list_cards::CardListResponse,
    move_card::MoveCardRequest,
    patch_card::PatchCardRequest,
    response::{
        CardDetailResponse, CardResponse, ColumnDto, CommentResponse, CriterionDto, EstimateDto,
        PriorityDto, ProvenanceResponse,
    },
    run_agent::{RunAgentRequest, RunAgentResponse},
    worksites::{
        LaunchWorksitesRequest, LaunchWorksitesResponse, LaunchedWorksiteResponse,
        WorksiteListResponse, WorksiteResponse,
    },
};
use capitainerie_handlers_clients::{
    self as clients,
    create_client::CreateClientRequest,
    create_project::CreateProjectRequest,
    response::{
        ClientDetailResponse, ClientListItemResponse, ClientListResponse, ClientResponse,
        ColumnCountResponse, ProjectDetailResponse, ProjectListItemResponse, ProjectListResponse,
        ProjectResponse,
    },
    update_client::UpdateClientRequest,
    update_project::UpdateProjectRequest,
};
use capitainerie_handlers_conversations::{
    self as conversations,
    dto::{
        ChatToolCallResponse, ConversationListResponse, ConversationResponse,
        ConversationSummaryResponse, CreateConversationRequest, CreatedConversationResponse,
        MessageResponse, MessageRoleDto, MessageStatusDto, ProposedCardResponse,
        SendMessageRequest, UpdateConversationRequest,
    },
};
use capitainerie_handlers_identity::{
    self as identity,
    list_members::{MemberListResponse, MemberResponse},
    me::{MeOrganization, MeResponse},
};
use capitainerie_handlers_inbox as inbox;
use capitainerie_handlers_integrations::{
    self as integrations, ActionAccessDto, ActionOutcomeDto, CheckIntegrationInstanceResponse,
    ConnectGithubRequest, CreateGithubAppRequest, CreateIntegrationInstanceRequest,
    CreateManifestRequest, GithubAppResponse, GithubInstallationResponse,
    GithubIntegrationResponse, InstallUrlResponse, InstanceFailureResponse,
    IntegrationActionListResponse, IntegrationActionResponse, IntegrationEnvironmentDto,
    IntegrationInstanceListResponse, IntegrationInstanceResponse, IntegrationKindDto,
    IssueListResponse, IssueResponse, ManifestOwner, ManifestResponse, RepositoryListResponse,
    RepositoryResponse, RevertIntegrationActionRequest, UpdateIntegrationInstanceRequest,
};
use capitainerie_handlers_mcp::{
    self as mcp,
    authorize::AuthorizeMcpConnectorResponse,
    callback::{McpOAuthCallbackRequest, McpOAuthCallbackResponse},
    create_connector::CreateMcpConnectorRequest,
    response::{McpConnectorListResponse, McpConnectorResponse, McpToolResponse},
    test_connector::TestMcpConnectorResponse,
    update_connector::UpdateMcpConnectorRequest,
};
use capitainerie_handlers_providers::{
    self as providers,
    dto::{
        CapabilitiesResponse, ModelAssignmentsDto, ModelAssignmentsResponse, ModelCatalogResponse,
        ModelRefDto, ModelResponse, PriceResponse, ProviderErrorResponse, ProviderKindDto,
        ProviderListResponse, ProviderResponse, UserProviderResponse,
    },
    put_provider::PutProviderRequest,
    put_user_provider::PutUserProviderRequest,
    test_provider::TestProviderResponse,
    test_user_provider::TestUserProviderResponse,
};
use capitainerie_handlers_routines::{
    self as routines,
    response::{
        AgentRunStatusDto, CardColumnDto, FrequencyDto, IfMissedDto, IfOpenDto,
        IssuedTriggerResponse, RoutineListResponse, RoutineResponse, RoutineRunListResponse,
        RoutineRunResponse, RoutineRunStatusDto, RunCountsResponse, RunOriginDto, ScheduleDto,
        TriggerKindDto, TriggerResponse,
    },
    routines::{CreateRoutineRequest, UpdateRoutineRequest},
    triggers::{AddTriggerRequest, UpdateTriggerRequest},
};
use capitainerie_handlers_runs::{
    self as runs,
    list_active_runs::{ActiveRunListResponse, ActiveRunResponse, ActiveStepResponse},
    list_runs::RunListResponse,
    response::{ClientRefResponse, ProjectRefResponse, RunResponse, RunStatusResponse},
    submit_transcript::{SubmitTranscriptRequest, SubmitTranscriptResponse},
    timeline::{
        DecisionResponse, RunTimelineResponse, StepResponse, StepStatusResponse, ToolCallResponse,
        UsageTotals,
    },
};
use capitainerie_handlers_skills::{
    self as skills,
    create_skill::CreateSkillRequest,
    response::{SkillListResponse, SkillResponse},
    update_skill::UpdateSkillRequest,
};
use capitainerie_handlers_sources::{
    self as sources,
    create_source::CreateSourceRequest,
    hook::HookResponse,
    response::{
        IssuedSourceResponse, SourceEventListResponse, SourceEventResponse, SourceEventStatusDto,
        SourceKindDto, SourceListResponse, SourceResponse,
    },
    update_source::UpdateSourceRequest,
};
use capitainerie_handlers_team::{
    self as team,
    create_agent::CreateTeamAgentRequest,
    response::{
        TeamAgentListResponse, TeamAgentResponse, TeamAgentRevisionListResponse,
        TeamAgentRevisionResponse, TeamAgentStatusDto,
    },
    update_agent::UpdateTeamAgentRequest,
};
use capitainerie_handlers_transcriptions::{
    self as transcriptions,
    create_transcription::{CreateTranscriptionRequest, TranscriptionResponse},
    read_document::{DocumentTextResponse, ReadDocumentRequest},
};
use utoipa::{
    Modify, OpenApi,
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
};

#[derive(OpenApi)]
#[openapi(
    info(title = "Capitainerie API"),
    paths(
        health::live,
        health::ready,
        identity::me::handler,
        identity::list_members::handler,
        runs::submit_transcript::handler,
        runs::list_runs::handler,
        runs::list_active_runs::handler,
        runs::get_run::handler,
        runs::timeline::handler,
        cards::list_cards::handler,
        cards::create_card::handler,
        cards::get_card::handler,
        cards::move_card::handler,
        cards::patch_card::handler,
        cards::comment_card::handler,
        cards::execute_card::handler,
        cards::intervene_card::handler,
        cards::run_agent::handler,
        cards::worksites::list_handler,
        cards::worksites::launch_handler,
        clients::list_clients::handler,
        clients::create_client::handler,
        clients::get_client::handler,
        clients::update_client::handler,
        clients::list_projects::handler,
        clients::create_project::handler,
        clients::get_project::handler,
        clients::update_project::handler,
        team::list_agents::handler,
        team::create_agent::handler,
        team::get_agent::handler,
        team::update_agent::handler,
        team::agent_revisions::list_handler,
        team::agent_revisions::restore_handler,
        skills::list_skills::handler,
        skills::create_skill::handler,
        skills::get_skill::handler,
        skills::update_skill::handler,
        skills::archive_skill::handler,
        approvals::list_approvals::handler,
        approvals::get_approval::handler,
        approvals::decide_approval::handler,
        audit::events::handler,
        audit::budgets::get_handler,
        audit::budgets::put_handler,
        audit::costs::projects_handler,
        audit::costs::agents_handler,
        audit::costs::tasks_handler,
        audit::costs::users_handler,
        inbox::list_handler,
        inbox::read_handler,
        artifacts::list_handler,
        artifacts::get_handler,
        sources::list_sources::handler,
        sources::create_source::handler,
        sources::get_source::handler,
        sources::update_source::handler,
        sources::archive_source::handler,
        sources::rotate_token::handler,
        sources::list_events::handler,
        sources::test_source::handler,
        sources::hook::handler,
        routines::routines::list_routines,
        routines::routines::create_routine,
        routines::routines::get_routine,
        routines::routines::update_routine,
        routines::routines::archive_routine,
        routines::runs::launch_routine,
        routines::runs::list_routine_runs,
        routines::triggers::add_trigger,
        routines::triggers::update_trigger,
        routines::triggers::remove_trigger,
        routines::triggers::rotate_trigger_token,
        routines::hook::trigger_routine,
        mcp::list_connectors::handler,
        mcp::create_connector::handler,
        mcp::get_connector::handler,
        mcp::update_connector::handler,
        mcp::authorize::handler,
        mcp::callback::handler,
        mcp::disconnect::handler,
        mcp::test_connector::handler,
        analytics::costs::handler,
        analytics::agents::handler,
        transcriptions::create_transcription::handler,
        transcriptions::read_document::handler,
        providers::list_providers::handler,
        providers::put_provider::handler,
        providers::delete_provider::handler,
        providers::test_provider::handler,
        providers::list_models::handler,
        providers::get_assignments::handler,
        providers::put_assignments::handler,
        providers::list_user_providers::handler,
        providers::put_user_provider::handler,
        providers::delete_user_provider::handler,
        providers::test_user_provider::handler,
        integrations::get_integration::handler,
        integrations::disconnect::handler,
        integrations::install_url::handler,
        integrations::callback::handler,
        integrations::create_manifest::handler,
        integrations::create_app::handler,
        integrations::delete_app::handler,
        integrations::list_repositories::handler,
        integrations::list_issues::handler,
        integrations::instances::list::handler,
        integrations::instances::create::handler,
        integrations::instances::update::handler,
        integrations::instances::archive::handler,
        integrations::instances::check::handler,
        integrations::actions::list_for_card::handler,
        integrations::actions::list_for_run::handler,
        integrations::actions::revert::handler,
        conversations::create_conversation::handler,
        conversations::list_conversations::handler,
        conversations::get_conversation::handler,
        conversations::update_conversation::handler,
        conversations::send_message::handler,
    ),
    components(schemas(
        ApiErrorResponse,
        MeResponse,
        MeOrganization,
        MemberListResponse,
        MemberResponse,
        SubmitTranscriptRequest,
        SubmitTranscriptResponse,
        RunListResponse,
        ActiveRunListResponse,
        ActiveRunResponse,
        ActiveStepResponse,
        RunResponse,
        RunStatusResponse,
        RunTimelineResponse,
        StepResponse,
        StepStatusResponse,
        ToolCallResponse,
        DecisionResponse,
        UsageTotals,
        CardListResponse,
        CardResponse,
        ColumnDto,
        PriorityDto,
        EstimateDto,
        CriterionDto,
        ProvenanceResponse,
        MoveCardRequest,
        PatchCardRequest,
        CreateCardRequest,
        ClientRefResponse,
        ProjectRefResponse,
        ClientResponse,
        ClientListItemResponse,
        ClientListResponse,
        ClientDetailResponse,
        ColumnCountResponse,
        CreateClientRequest,
        UpdateClientRequest,
        ProjectResponse,
        ProjectListItemResponse,
        ProjectListResponse,
        ProjectDetailResponse,
        CreateProjectRequest,
        UpdateProjectRequest,
        CardDetailResponse,
        CommentResponse,
        CommentCardRequest,
        ExecuteCardRequest,
        ExecuteCardResponse,
        LaunchWorksitesRequest,
        LaunchWorksitesResponse,
        LaunchedWorksiteResponse,
        WorksiteListResponse,
        WorksiteResponse,
        IssueListResponse,
        IssueResponse,
        GrantAccessDto,
        InstanceGrantRequest,
        InterveneCardRequest,
        InterveneCardResponse,
        RunAgentRequest,
        RunAgentResponse,
        CostsResponse,
        CostRowResponse,
        CostTotals,
        CostDimensionParam,
        AgentResponse,
        McpConnectorResponse,
        McpConnectorListResponse,
        McpToolResponse,
        CreateMcpConnectorRequest,
        UpdateMcpConnectorRequest,
        AuthorizeMcpConnectorResponse,
        McpOAuthCallbackRequest,
        McpOAuthCallbackResponse,
        TestMcpConnectorResponse,
        TeamAgentResponse,
        TeamAgentListResponse,
        TeamAgentStatusDto,
        CreateTeamAgentRequest,
        UpdateTeamAgentRequest,
        TeamAgentRevisionResponse,
        TeamAgentRevisionListResponse,
        SkillResponse,
        SkillListResponse,
        CreateSkillRequest,
        UpdateSkillRequest,
        ApprovalResponse,
        ApprovalListResponse,
        DecideApprovalResponse,
        ApprovalKindDto,
        ApprovalStatusDto,
        ApprovalDecisionDto,
        DecideApprovalRequest,
        SourceResponse,
        SourceListResponse,
        SourceKindDto,
        IssuedSourceResponse,
        SourceEventResponse,
        SourceEventListResponse,
        SourceEventStatusDto,
        CreateSourceRequest,
        UpdateSourceRequest,
        HookResponse,
        RoutineResponse,
        RoutineListResponse,
        RoutineRunResponse,
        RoutineRunListResponse,
        RunCountsResponse,
        TriggerResponse,
        IssuedTriggerResponse,
        ScheduleDto,
        FrequencyDto,
        TriggerKindDto,
        IfOpenDto,
        IfMissedDto,
        RunOriginDto,
        RoutineRunStatusDto,
        CardColumnDto,
        AgentRunStatusDto,
        CreateRoutineRequest,
        UpdateRoutineRequest,
        AddTriggerRequest,
        UpdateTriggerRequest,
        CreateTranscriptionRequest,
        TranscriptionResponse,
        ReadDocumentRequest,
        DocumentTextResponse,
        ProviderKindDto,
        ProviderResponse,
        ProviderListResponse,
        PutProviderRequest,
        PutUserProviderRequest,
        UserProviderResponse,
        TestUserProviderResponse,
        TestProviderResponse,
        ModelResponse,
        CapabilitiesResponse,
        PriceResponse,
        ProviderErrorResponse,
        ModelCatalogResponse,
        ModelRefDto,
        ModelAssignmentsDto,
        ModelAssignmentsResponse,
        GithubIntegrationResponse,
        GithubAppResponse,
        GithubInstallationResponse,
        CreateManifestRequest,
        ManifestOwner,
        ManifestResponse,
        CreateGithubAppRequest,
        InstallUrlResponse,
        ConnectGithubRequest,
        RepositoryListResponse,
        RepositoryResponse,
        IntegrationKindDto,
        IntegrationEnvironmentDto,
        IntegrationInstanceResponse,
        IntegrationInstanceListResponse,
        CreateIntegrationInstanceRequest,
        UpdateIntegrationInstanceRequest,
        CheckIntegrationInstanceResponse,
        InstanceFailureResponse,
        ActionAccessDto,
        ActionOutcomeDto,
        IntegrationActionResponse,
        IntegrationActionListResponse,
        RevertIntegrationActionRequest,
        CreateConversationRequest,
        CreatedConversationResponse,
        UpdateConversationRequest,
        SendMessageRequest,
        ConversationListResponse,
        ConversationResponse,
        ConversationSummaryResponse,
        MessageResponse,
        MessageRoleDto,
        MessageStatusDto,
        ProposedCardResponse,
        ChatToolCallResponse,
    )),
    modifiers(&BearerAuth),
    tags(
        (name = health::TAG, description = "Liveness and readiness probes"),
        (name = identity::TAG, description = "The authenticated caller"),
        (name = runs::TAG, description = "Transcripts and the runs processing them"),
        (name = cards::TAG, description = "The Kanban board and its human validation"),
        (name = clients::TAG, description = "The clients the team works for, their projects, and the link of a card to them"),
        (name = team::TAG, description = "The registry of the team's agents: their role, model, instructions, connector grants and reporting line"),
        (name = skills::TAG, description = "The organization's catalog of skills: instructions added to the system prompt of the agents whose skills list names them"),
        (name = approvals::TAG, description = "Decisions agents ask the team to take, such as a write a run was not allowed to make"),
        (name = audit::TAG, description = "Who did what, from the organization's event log, and the monthly budgets set against the agents' spend"),
        (name = inbox::TAG, description = "Each member's notifications, projected from the event log: approvals to give, agent runs they started, cards assigned or commented by an agent, failed sources and routines"),
        (name = artifacts::TAG, description = "Deliverables the agents produced, such as the final report of an agent run"),
        (name = sources::TAG, description = "External inputs whose events become cards handled by an agent: each source has a secret webhook URL, and the API keeps only a hash of its token"),
        (name = routines::TAG, description = "Recurring work an agent does: on a planning or a webhook, each passage creates a card assigned to the agent and starts its run on behalf of the routine's creator"),
        (name = mcp::TAG, description = "MCP servers the organization connects to, each user authorizing with their own OAuth2 session (authorization code, PKCE S256, dynamic client registration)"),
        (name = analytics::TAG, description = "Costs and activity of the agents"),
        (name = providers::TAG, description = "Model providers, their catalogs and the model of each agent role"),
        (name = transcriptions::TAG, description = "Audio and PDF files turned into text for the transcript form; the file is never kept"),
        (name = conversations::TAG, description = "The chat with the Captain: conversations, their messages and the streamed turns"),
        (name = integrations::TAG, description = "The GitHub App each organization creates, and its installation, that let Claude Code work on a repository; the Odoo and n8n instances of each client"),
    )
)]
pub struct ApiDoc;

struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_operation_has_an_operation_id() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = spec["paths"].as_object().unwrap();
        for (path, operations) in paths {
            for (method, operation) in operations.as_object().unwrap() {
                assert!(
                    operation["operationId"].is_string(),
                    "{method} {path} lacks an operationId"
                );
            }
        }
    }

    #[test]
    fn path_parameters_appear_in_their_path() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        for (path, operations) in spec["paths"].as_object().unwrap() {
            for (method, operation) in operations.as_object().unwrap() {
                let Some(parameters) = operation["parameters"].as_array() else {
                    continue;
                };
                for parameter in parameters.iter().filter(|p| p["in"] == "path") {
                    let name = parameter["name"].as_str().unwrap();
                    assert!(
                        path.contains(&format!("{{{name}}}")),
                        "{method} {path} declares path parameter {name} absent from the path"
                    );
                }
            }
        }
    }

    #[test]
    fn analytics_endpoints_document_their_query_parameters() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let costs = &spec["paths"]["/api/v1/analytics/costs"]["get"];
        let names: Vec<_> = costs["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["in"] == "query")
            .map(|p| p["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["by", "from", "to"]);
        assert!(spec["paths"]["/api/v1/agents"]["get"].is_object());
    }

    #[test]
    fn the_card_panel_endpoints_are_documented() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        assert!(paths["/api/v1/cards/{card_id}"]["get"].is_object());
        assert!(paths["/api/v1/cards/{card_id}/comments"]["post"].is_object());
        let detail = &spec["components"]["schemas"]["CardDetailResponse"]["properties"];
        for field in ["card", "comments", "runs"] {
            assert!(detail[field].is_object(), "{field}");
        }
        let comment = &spec["components"]["schemas"]["CommentResponse"]["properties"];
        for field in ["id", "body", "author_subject", "author_actor", "created_at"] {
            assert!(comment[field].is_object(), "{field}");
        }
    }

    #[test]
    fn the_client_and_project_endpoints_are_documented() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        assert!(paths["/api/v1/clients"]["get"].is_object());
        assert!(paths["/api/v1/clients"]["post"].is_object());
        assert!(paths["/api/v1/clients/{client_id}"]["get"].is_object());
        assert!(paths["/api/v1/clients/{client_id}"]["patch"].is_object());
        assert!(paths["/api/v1/projects"]["get"].is_object());
        assert!(paths["/api/v1/projects"]["post"].is_object());
        assert!(paths["/api/v1/projects/{project_id}"]["get"].is_object());
        assert!(paths["/api/v1/projects/{project_id}"]["patch"].is_object());
        assert!(paths["/api/v1/cards/{card_id}"]["patch"].is_object());
        let queries = |path: &str| -> Vec<String> {
            paths[path]["get"]["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["in"] == "query")
                .map(|p| p["name"].as_str().unwrap().to_owned())
                .collect()
        };
        assert_eq!(queries("/api/v1/clients"), ["q", "archived"]);
        assert_eq!(queries("/api/v1/projects"), ["q", "archived", "client_id"]);
        let schemas = &spec["components"]["schemas"];
        for schema in ["CardResponse", "RunResponse"] {
            for field in ["client", "project"] {
                assert!(
                    schemas[schema]["properties"][field].is_object(),
                    "{schema}.{field}"
                );
            }
        }
        let patch = &schemas["PatchCardRequest"]["properties"];
        for field in ["expected_version", "client_id", "project_id"] {
            assert!(patch[field].is_object(), "{field}");
        }
        for field in ["client_id", "project_id"] {
            assert!(
                schemas["SubmitTranscriptRequest"]["properties"][field].is_object(),
                "{field}"
            );
        }
        let detail = &schemas["ClientDetailResponse"]["properties"];
        for field in ["client", "projects", "cards"] {
            assert!(detail[field].is_object(), "{field}");
        }
    }

    #[test]
    fn the_sources_and_their_public_hook_are_documented() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        assert!(paths["/api/v1/sources"]["get"].is_object());
        assert!(paths["/api/v1/sources"]["post"].is_object());
        assert!(paths["/api/v1/sources/{source_id}"]["get"].is_object());
        assert!(paths["/api/v1/sources/{source_id}"]["patch"].is_object());
        assert!(paths["/api/v1/sources/{source_id}/archive"]["post"].is_object());
        assert!(paths["/api/v1/sources/{source_id}/rotate-token"]["post"].is_object());
        assert!(paths["/api/v1/sources/{source_id}/events"]["get"].is_object());
        assert!(paths["/api/v1/sources/{source_id}/test"]["post"].is_object());
        let hook = &paths["/hooks/sources/{token}"]["post"];
        assert!(hook.is_object());
        assert!(hook["security"].is_null());
        let schemas = &spec["components"]["schemas"];
        assert!(schemas["IssuedSourceResponse"]["properties"]["webhook_url"].is_object());
        assert!(schemas["SourceResponse"]["properties"]["token"].is_null());
        assert!(schemas["SourceResponse"]["properties"]["events_last_24h"].is_object());
    }

    #[test]
    fn the_routines_and_their_public_hook_are_documented() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        assert!(paths["/api/v1/routines"]["get"].is_object());
        assert!(paths["/api/v1/routines"]["post"].is_object());
        assert!(paths["/api/v1/routines/{routine_id}"]["get"].is_object());
        assert!(paths["/api/v1/routines/{routine_id}"]["patch"].is_object());
        assert!(paths["/api/v1/routines/{routine_id}/archive"]["post"].is_object());
        assert!(paths["/api/v1/routines/{routine_id}/launch"]["post"].is_object());
        assert!(paths["/api/v1/routines/{routine_id}/runs"]["get"].is_object());
        assert!(paths["/api/v1/routines/{routine_id}/triggers"]["post"].is_object());
        let trigger = &paths["/api/v1/routines/{routine_id}/triggers/{trigger_id}"];
        assert!(trigger["patch"].is_object());
        assert!(trigger["delete"].is_object());
        assert!(
            paths["/api/v1/routines/{routine_id}/triggers/{trigger_id}/rotate-token"]["post"]
                .is_object()
        );
        let hook = &paths["/hooks/routines/{token}"]["post"];
        assert!(hook.is_object());
        assert!(hook["security"].is_null());
        let schemas = &spec["components"]["schemas"];
        assert!(schemas["RoutineResponse"]["properties"]["next_run_at"].is_object());
        assert!(schemas["IssuedTriggerResponse"]["properties"]["webhook_url"].is_object());
    }

    #[test]
    fn the_skills_and_approvals_are_documented() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        assert!(paths["/api/v1/skills"]["get"].is_object());
        assert!(paths["/api/v1/skills"]["post"].is_object());
        assert!(paths["/api/v1/skills/{skill_id}"]["get"].is_object());
        assert!(paths["/api/v1/skills/{skill_id}"]["patch"].is_object());
        assert!(paths["/api/v1/skills/{skill_id}"]["delete"].is_null());
        assert!(paths["/api/v1/skills/{skill_id}/archive"]["post"].is_object());
        assert!(paths["/api/v1/approvals"]["get"].is_object());
        assert!(paths["/api/v1/approvals/{approval_id}"]["get"].is_object());
        assert!(paths["/api/v1/approvals/{approval_id}/decide"]["post"].is_object());
        let schemas = &spec["components"]["schemas"];
        assert!(schemas["SkillResponse"]["properties"]["used_by"].is_object());
        assert!(schemas["ApprovalListResponse"]["properties"]["pending"].is_object());
        assert!(schemas["DecideApprovalRequest"]["properties"]["decision"].is_object());
        assert!(schemas["ApprovalResponse"]["properties"]["resumed_run_id"].is_object());
        assert!(schemas["DecideApprovalResponse"].is_object());
    }

    #[test]
    fn the_audit_inbox_and_artifacts_are_documented() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        assert!(paths["/api/v1/audit/events"]["get"].is_object());
        assert!(paths["/api/v1/budgets"]["get"].is_object());
        assert!(paths["/api/v1/budgets"]["put"].is_object());
        for route in ["projects", "agents", "tasks", "users"] {
            assert!(paths[format!("/api/v1/analytics/costs/{route}")]["get"].is_object());
        }
        assert!(paths["/api/v1/inbox"]["get"].is_object());
        assert!(paths["/api/v1/inbox/read"]["post"].is_object());
        assert!(paths["/api/v1/artifacts"]["get"].is_object());
        assert!(paths["/api/v1/artifacts/{artifact_id}"]["get"].is_object());
        let schemas = &spec["components"]["schemas"];
        assert!(schemas["AuditEventResponse"]["properties"]["summary"].is_object());
        assert!(schemas["BudgetsResponse"]["properties"]["spent_micros"].is_object());
        assert!(schemas["ProjectCostsResponse"]["properties"]["rows"].is_object());
        assert!(schemas["InboxResponse"]["properties"]["unread"].is_object());
        assert!(schemas["ArtifactListResponse"]["properties"]["next_cursor"].is_object());
        assert!(schemas["RunResponse"]["properties"]["agent"].is_object());
    }

    #[test]
    fn the_team_registry_and_card_creation_are_documented() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        assert!(paths["/api/v1/team/agents"]["get"].is_object());
        assert!(paths["/api/v1/team/agents"]["post"].is_object());
        assert!(paths["/api/v1/team/agents/{agent_id}"]["get"].is_object());
        assert!(paths["/api/v1/team/agents/{agent_id}"]["patch"].is_object());
        assert!(paths["/api/v1/team/agents/{agent_id}"]["delete"].is_null());
        assert!(paths["/api/v1/team/agents/{agent_id}/revisions"]["get"].is_object());
        assert!(
            paths["/api/v1/team/agents/{agent_id}/revisions/{revision}/restore"]["post"]
                .is_object()
        );
        let card_filters: Vec<&str> = paths["/api/v1/cards"]["get"]["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|parameter| parameter["name"].as_str())
            .collect();
        for filter in ["column", "status", "assignee_agent_id", "priority"] {
            assert!(card_filters.contains(&filter), "{filter}");
        }
        assert!(paths["/api/v1/agents"]["get"].is_object());
        assert!(paths["/api/v1/cards"]["post"].is_object());
        let schemas = &spec["components"]["schemas"];
        for field in ["assignee_agent_id", "parent_id"] {
            for schema in ["CardResponse", "PatchCardRequest", "CreateCardRequest"] {
                assert!(
                    schemas[schema]["properties"][field].is_object(),
                    "{schema}.{field}"
                );
            }
        }
        for field in [
            "connector_grants",
            "repository_grants",
            "reports_to",
            "instructions",
            "status",
            "config",
            "skills",
            "paused",
            "revision",
        ] {
            assert!(
                schemas["TeamAgentResponse"]["properties"][field].is_object(),
                "{field}"
            );
        }
    }

    #[test]
    fn the_mcp_connector_endpoints_are_documented_without_a_token_in_any_response() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        assert!(paths["/api/v1/mcp-connectors"]["get"].is_object());
        assert!(paths["/api/v1/mcp-connectors"]["post"].is_object());
        assert!(paths["/api/v1/mcp-connectors/{connector_id}"]["get"].is_object());
        assert!(paths["/api/v1/mcp-connectors/{connector_id}"]["patch"].is_object());
        assert!(paths["/api/v1/mcp-connectors/{connector_id}/authorize"]["post"].is_object());
        assert!(paths["/api/v1/mcp-connectors/oauth/callback"]["post"].is_object());
        assert!(paths["/api/v1/mcp-connectors/{connector_id}/connection"]["delete"].is_object());
        assert!(paths["/api/v1/mcp-connectors/{connector_id}/test"]["post"].is_object());
        let connector = &spec["components"]["schemas"]["McpConnectorResponse"]["properties"];
        for field in ["connected", "expires_at", "resource", "scopes", "slug"] {
            assert!(connector[field].is_object(), "{field}");
        }
        for secret in [
            "access_token",
            "refresh_token",
            "client_id",
            "code_verifier",
        ] {
            assert!(connector.get(secret).is_none(), "{secret}");
        }
    }

    #[test]
    fn the_provider_endpoints_are_documented_without_a_key_in_any_response() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        assert!(paths["/api/v1/providers"]["get"].is_object());
        assert!(paths["/api/v1/providers/{kind}"]["put"].is_object());
        assert!(paths["/api/v1/providers/{kind}"]["delete"].is_object());
        assert!(paths["/api/v1/providers/{kind}/test"]["post"].is_object());
        assert!(paths["/api/v1/models"]["get"].is_object());
        assert!(paths["/api/v1/model-assignments"]["get"].is_object());
        assert!(paths["/api/v1/model-assignments"]["put"].is_object());
        assert!(paths["/api/v1/me/providers"]["get"].is_object());
        assert!(paths["/api/v1/me/providers/{kind}"]["put"].is_object());
        assert!(paths["/api/v1/me/providers/{kind}"]["delete"].is_object());
        assert!(paths["/api/v1/me/providers/{kind}/test"]["post"].is_object());
        let mine = &spec["components"]["schemas"]["UserProviderResponse"]["properties"];
        assert!(mine.get("api_key").is_none());
        assert!(mine["preview"].is_object());
        assert!(mine["is_default"].is_object());
        let provider = &spec["components"]["schemas"]["ProviderResponse"]["properties"];
        assert!(provider.get("api_key").is_none());
        assert!(provider["preview"].is_object());
    }

    #[test]
    fn the_transcription_endpoint_is_documented_as_multipart() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let operation = &spec["paths"]["/api/v1/transcriptions"]["post"];
        assert_eq!(operation["operationId"], "createTranscription");
        let content = &operation["requestBody"]["content"]["multipart/form-data"];
        assert!(content["schema"].is_object());
        for status in ["200", "409", "413", "415", "502"] {
            assert!(operation["responses"][status].is_object(), "{status}");
        }
        let schemas = &spec["components"]["schemas"];
        assert!(schemas["TranscriptionResponse"]["properties"]["text"].is_object());
        assert!(schemas["TranscriptionResponse"]["properties"]["model"].is_object());
        assert!(schemas["CapabilitiesResponse"]["properties"]["transcription"].is_object());
        assert!(schemas["ModelAssignmentsDto"]["properties"]["transcriber"].is_object());
    }

    #[test]
    fn the_document_endpoint_is_documented_as_multipart() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let operation = &spec["paths"]["/api/v1/documents/text"]["post"];
        assert_eq!(operation["operationId"], "readDocument");
        assert!(operation["requestBody"]["content"]["multipart/form-data"]["schema"].is_object());
        for status in ["200", "409", "413", "415", "502"] {
            assert!(operation["responses"][status].is_object(), "{status}");
        }
        let schemas = &spec["components"]["schemas"];
        for field in ["text", "model", "pages"] {
            assert!(
                schemas["DocumentTextResponse"]["properties"][field].is_object(),
                "{field}"
            );
        }
        assert!(schemas["CapabilitiesResponse"]["properties"]["ocr"].is_object());
        assert!(schemas["ModelAssignmentsDto"]["properties"]["reader"].is_object());
    }

    #[test]
    fn the_execution_and_github_endpoints_are_documented() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        assert!(paths["/api/v1/cards/{card_id}/executions"]["post"].is_object());
        assert!(paths["/api/v1/integrations/github"]["get"].is_object());
        assert!(paths["/api/v1/integrations/github"]["delete"].is_object());
        assert!(paths["/api/v1/integrations/github/install"]["get"].is_object());
        assert!(paths["/api/v1/integrations/github/callback"]["post"].is_object());
        assert!(paths["/api/v1/integrations/github/repositories"]["get"].is_object());
        let schemas = &spec["components"]["schemas"];
        assert!(schemas["ExecuteCardResponse"]["properties"]["run_id"].is_object());
        assert!(schemas["GithubIntegrationResponse"]["properties"]["app"].is_object());
        assert!(schemas["GithubIntegrationResponse"]["properties"]["installation"].is_object());
        assert!(paths["/api/v1/integrations/github/app/manifest"]["post"].is_object());
        assert!(paths["/api/v1/integrations/github/app"]["post"].is_object());
        assert!(paths["/api/v1/integrations/github/app"]["delete"].is_object());
        assert!(schemas["ManifestResponse"]["properties"]["action_url"].is_object());
        let app = &schemas["GithubAppResponse"]["properties"];
        for secret in ["client_secret", "pem", "private_key", "webhook_secret"] {
            assert!(app.get(secret).is_none(), "{secret}");
        }
    }

    fn referenced_schemas<'a>(
        spec: &'a serde_json::Value,
        node: &'a serde_json::Value,
        seen: &mut std::collections::BTreeSet<&'a str>,
    ) {
        match node {
            serde_json::Value::Object(map) => {
                if let Some(reference) = map.get("$ref").and_then(|value| value.as_str()) {
                    let name = reference.rsplit('/').next().unwrap();
                    if seen.insert(name) {
                        referenced_schemas(spec, &spec["components"]["schemas"][name], seen);
                    }
                }
                for value in map.values() {
                    referenced_schemas(spec, value, seen);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    referenced_schemas(spec, item, seen);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn the_instance_endpoints_are_documented_without_a_secret_in_any_response() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        let operations = [
            (
                "/api/v1/clients/{client_id}/integrations",
                "get",
                "listIntegrationInstances",
            ),
            (
                "/api/v1/clients/{client_id}/integrations",
                "post",
                "createIntegrationInstance",
            ),
            (
                "/api/v1/integration-instances/{id}",
                "patch",
                "updateIntegrationInstance",
            ),
            (
                "/api/v1/integration-instances/{id}",
                "delete",
                "archiveIntegrationInstance",
            ),
            (
                "/api/v1/integration-instances/{id}/check",
                "post",
                "checkIntegrationInstance",
            ),
        ];
        for (path, method, operation_id) in operations {
            let operation = &paths[path][method];
            assert_eq!(operation["operationId"], operation_id, "{method} {path}");
            let mut reachable = std::collections::BTreeSet::new();
            referenced_schemas(&spec, &operation["responses"], &mut reachable);
            assert!(!reachable.is_empty(), "{method} {path}");
            for name in reachable {
                let properties = &spec["components"]["schemas"][name]["properties"];
                assert!(
                    properties.get("secret").is_none(),
                    "{name} reachable from {method} {path} has a secret property"
                );
            }
        }
        let instance = &spec["components"]["schemas"]["IntegrationInstanceResponse"]["properties"];
        assert!(instance["secret_preview"].is_object());
        assert!(
            spec["components"]["schemas"]["CreateIntegrationInstanceRequest"]["properties"]["secret"]
                .is_object()
        );
        let check =
            &spec["components"]["schemas"]["CheckIntegrationInstanceResponse"]["properties"];
        for field in ["ok", "server_version", "warning", "error", "instance"] {
            assert!(check[field].is_object(), "{field}");
        }
        assert!(
            paths["/api/v1/clients/{client_id}/integrations"]["get"]["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .any(|parameter| parameter["in"] == "query" && parameter["name"] == "archived")
        );
        for status in ["201", "403", "404", "409", "422"] {
            assert!(
                paths["/api/v1/clients/{client_id}/integrations"]["post"]["responses"][status]
                    .is_object(),
                "{status}"
            );
        }
    }

    #[test]
    fn the_agent_run_endpoint_is_documented() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let run = &spec["paths"]["/api/v1/cards/{card_id}/agent-runs"]["post"];
        assert_eq!(run["operationId"], "runCardAgent");
        for status in ["202", "401", "403", "404", "409", "422"] {
            assert!(run["responses"][status].is_object(), "{status}");
        }
        let request = &spec["components"]["schemas"]["RunAgentRequest"];
        assert!(request["properties"]["expected_version"].is_object());
        assert!(request["properties"]["allow_writes"].is_object());
        let required: Vec<_> = request["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|field| field.as_str().unwrap())
            .collect();
        assert_eq!(required, ["expected_version"]);
        assert!(
            spec["components"]["schemas"]["RunAgentResponse"]["properties"]["run_id"].is_object()
        );
    }

    #[test]
    fn the_intervention_and_journal_endpoints_are_documented_without_a_snapshot() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        let schemas = &spec["components"]["schemas"];

        let confide = &paths["/api/v1/cards/{card_id}/interventions"]["post"];
        assert_eq!(confide["operationId"], "interveneCard");
        for status in ["202", "403", "404", "409", "422"] {
            assert!(confide["responses"][status].is_object(), "{status}");
        }
        let request = &schemas["InterveneCardRequest"];
        for field in [
            "instructions",
            "grants",
            "confirm_production_writes",
            "expected_version",
        ] {
            assert!(request["properties"][field].is_object(), "{field}");
        }
        let required: Vec<_> = request["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|field| field.as_str().unwrap())
            .collect();
        assert!(required.contains(&"grants"));
        assert!(required.contains(&"expected_version"));
        assert!(!required.contains(&"instructions"));
        assert!(!required.contains(&"confirm_production_writes"));
        let grant = &schemas["InstanceGrantRequest"]["properties"];
        assert!(grant["instance_id"].is_object());
        assert!(grant["access"].is_object());
        for field in ["run_id", "card"] {
            assert!(
                schemas["InterveneCardResponse"]["properties"][field].is_object(),
                "{field}"
            );
        }

        let journal = [
            (
                "/api/v1/cards/{card_id}/integration-actions",
                "listCardIntegrationActions",
            ),
            (
                "/api/v1/runs/{run_id}/integration-actions",
                "listRunIntegrationActions",
            ),
        ];
        for (path, operation_id) in journal {
            let operation = &paths[path]["get"];
            assert_eq!(operation["operationId"], operation_id, "{path}");
            for status in ["200", "403", "404"] {
                assert!(
                    operation["responses"][status].is_object(),
                    "{path} {status}"
                );
            }
            let mut reachable = std::collections::BTreeSet::new();
            referenced_schemas(&spec, &operation["responses"], &mut reachable);
            assert!(reachable.contains("IntegrationActionResponse"), "{path}");
            for name in reachable {
                let properties = &schemas[name]["properties"];
                assert!(
                    properties.get("snapshot").is_none(),
                    "{name} reachable from {path} has a snapshot property"
                );
            }
        }
        let action = &schemas["IntegrationActionResponse"]["properties"];
        for field in [
            "id",
            "run_id",
            "card_id",
            "instance_id",
            "instance_name",
            "instance_environment",
            "instance_kind",
            "actor",
            "tool",
            "access",
            "target",
            "arguments",
            "outcome",
            "reason_code",
            "reason",
            "before",
            "after",
            "revertible",
            "reverts_action_id",
            "reverted_by_action_id",
            "latency_ms",
            "created_at",
            "finished_at",
        ] {
            assert!(action[field].is_object(), "{field}");
        }
        assert!(schemas["IntegrationActionListResponse"]["properties"]["items"].is_object());
    }

    #[test]
    fn the_revert_endpoint_is_documented_with_its_refusals_and_without_a_snapshot() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let schemas = &spec["components"]["schemas"];
        let revert = &spec["paths"]["/api/v1/integration-actions/{id}/revert"]["post"];

        assert_eq!(revert["operationId"], "revertIntegrationAction");
        for status in ["200", "401", "403", "404", "409", "422"] {
            assert!(revert["responses"][status].is_object(), "{status}");
        }
        assert!(
            schemas["RevertIntegrationActionRequest"]["properties"]["confirm_production"]
                .is_object()
        );
        let required = schemas["RevertIntegrationActionRequest"]["required"].as_array();
        assert!(required.is_none_or(|fields| fields.is_empty()));
        let mut reachable = std::collections::BTreeSet::new();
        referenced_schemas(&spec, &revert["responses"], &mut reachable);
        assert!(reachable.contains("IntegrationActionListResponse"));
        assert!(reachable.contains("IntegrationActionResponse"));
        for name in reachable {
            assert!(
                schemas[name]["properties"].get("snapshot").is_none(),
                "{name} has a snapshot property"
            );
        }
    }

    #[test]
    fn the_chat_endpoints_are_documented_with_their_stream() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let paths = &spec["paths"];
        assert_eq!(
            paths["/api/v1/conversations"]["post"]["operationId"],
            "createConversation"
        );
        assert!(paths["/api/v1/conversations"]["get"].is_object());
        assert!(paths["/api/v1/conversations/{id}"]["get"].is_object());
        assert!(paths["/api/v1/conversations/{id}"]["patch"].is_object());
        let turn = &paths["/api/v1/conversations/{id}/messages"]["post"];
        assert_eq!(turn["operationId"], "sendConversationMessage");
        assert!(turn["responses"]["200"]["content"]["text/event-stream"].is_object());
        for status in ["403", "404", "409"] {
            assert!(turn["responses"][status].is_object(), "{status}");
        }
        let schemas = &spec["components"]["schemas"];
        for field in ["conversation", "messages"] {
            assert!(
                schemas["ConversationResponse"]["properties"][field].is_object(),
                "{field}"
            );
        }
        assert!(schemas["ModelAssignmentsDto"]["properties"]["captain"].is_object());
        assert!(schemas["ToolCallResponse"].is_object());
        assert!(schemas["ChatToolCallResponse"]["properties"]["summary"].is_object());
    }
}
