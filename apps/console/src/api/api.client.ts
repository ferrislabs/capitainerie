export namespace Schemas {
  // <Schemas>
  export type ActionAccessDto = "read" | "write";
  export type ActionOutcomeDto = "pending" | "succeeded" | "denied" | "failed";
  export type ActiveStepResponse = {
    agent?: (string | null) | undefined;
    kind: string;
    model?: (string | null) | undefined;
    started_at: string;
  };
  export type AgentId = string;
  export type RunAgentRef = { id: AgentId; name: string };
  export type CardId = string;
  export type ClientId = string;
  export type ClientRefResponse = { id: ClientId; name: string };
  export type RunId = string;
  export type ProjectId = string;
  export type ProjectRefResponse = { id: ProjectId; name: string };
  export type RunStatusResponse = "queued" | "running" | "succeeded" | "failed" | "cancelled";
  export type TranscriptId = string;
  export type RunResponse = {
    agent?: (null | RunAgentRef) | undefined;
    card_id?: (null | CardId) | undefined;
    client?: (null | ClientRefResponse) | undefined;
    created_at: string;
    failure?: (string | null) | undefined;
    finished_at?: (string | null) | undefined;
    id: RunId;
    initiated_by: string;
    pipeline: Array<string>;
    project?: (null | ProjectRefResponse) | undefined;
    repository?: (string | null) | undefined;
    repository_hint?: (string | null) | undefined;
    started_at?: (string | null) | undefined;
    status: RunStatusResponse;
    title?: (string | null) | undefined;
    trace_id?: (string | null) | undefined;
    transcript_id?: (null | TranscriptId) | undefined;
    transcript_title?: (string | null) | undefined;
  };
  export type ActiveRunResponse = {
    cost_micros: number;
    current_step?: (null | ActiveStepResponse) | undefined;
    input_tokens: number;
    output_tokens: number;
    run: RunResponse;
    steps_done: number;
    steps_total: number;
  };
  export type ActiveRunListResponse = { items: Array<ActiveRunResponse> };
  export type ActorKindDto = "member" | "agent" | "system";
  export type TriggerKindDto = "planning" | "webhook";
  export type FrequencyDto = "every_hours" | "daily" | "weekly" | "monthly" | "cron";
  export type ScheduleDto = {
    cron?: (string | null) | undefined;
    days?: Array<string> | undefined;
    frequency: FrequencyDto;
    hours?: (number | null) | undefined;
    time?: (string | null) | undefined;
  };
  export type AddTriggerRequest = {
    enabled?: (boolean | null) | undefined;
    kind: TriggerKindDto;
    schedule?: (null | ScheduleDto) | undefined;
  };
  export type AgentSpendResponse = {
    agent_id?: (null | AgentId) | undefined;
    cost_micros: number;
    input_tokens: number;
    name: string;
    output_tokens: number;
    runs: number;
  };
  export type SpentResponse = { cost_micros: number; input_tokens: number; output_tokens: number };
  export type AgentCostsResponse = { from: string; rows: Array<AgentSpendResponse>; to: string; total: SpentResponse };
  export type AgentLimitRequest = {
    agent_id: AgentId;
    alert_threshold_percent?: (number | null) | undefined;
    monthly_limit_cents: number;
  };
  export type AgentResponse = {
    agent: string;
    cost_micros: number;
    input_tokens: number;
    last_seen: string;
    output_tokens: number;
    runs: number;
  };
  export type AgentRunStatusDto = "queued" | "running" | "succeeded" | "failed" | "cancelled";
  export type ApiErrorResponse = { code: string; message: string; status: number };
  export type ApprovalDecisionDto = "approve" | "reject";
  export type ApprovalId = string;
  export type ApprovalKindDto = "tool_call" | "agent_action";
  export type ApprovalStatusDto = "pending" | "approved" | "rejected" | "expired";
  export type ApprovalResponse = {
    card_id?: (null | CardId) | undefined;
    created_at: string;
    decided_at?: (string | null) | undefined;
    decided_by?: (string | null) | undefined;
    decision_note?: (string | null) | undefined;
    detail: Record<string, unknown>;
    id: ApprovalId;
    kind: ApprovalKindDto;
    requested_by_agent_id?: (null | AgentId) | undefined;
    resumed_run_id?: (null | RunId) | undefined;
    run_id?: (null | RunId) | undefined;
    status: ApprovalStatusDto;
    title: string;
  };
  export type ApprovalListResponse = { items: Array<ApprovalResponse>; pending: number };
  export type ArtifactId = string;
  export type ArtifactKindDto = "markdown" | "json" | "link";
  export type ArtifactSummaryResponse = {
    agent_id?: (null | AgentId) | undefined;
    agent_name?: (string | null) | undefined;
    card_id?: (null | CardId) | undefined;
    card_title?: (string | null) | undefined;
    created_at: string;
    created_by: string;
    id: ArtifactId;
    kind: ArtifactKindDto;
    run_id?: (null | RunId) | undefined;
    title: string;
    updated_at: string;
    url?: (string | null) | undefined;
  };
  export type ArtifactListResponse = {
    items: Array<ArtifactSummaryResponse>;
    next_cursor?: (null | ArtifactId) | undefined;
  };
  export type ArtifactResponse = ArtifactSummaryResponse & Partial<{ content: string | null }>;
  export type AuditActorResponse = { id?: (string | null) | undefined; kind: ActorKindDto; name: string };
  export type SeverityDto = "critical" | "warning" | "info" | "success" | "created";
  export type AuditEventResponse = {
    actor: AuditActorResponse;
    card_id?: (string | null) | undefined;
    id: string;
    occurred_at: string;
    payload: Record<string, unknown>;
    run_id?: (string | null) | undefined;
    severity: SeverityDto;
    summary: string;
    topic: string;
    type: string;
  };
  export type AuditEventListResponse = { items: Array<AuditEventResponse>; next_cursor?: (string | null) | undefined };
  export type AuthorizeMcpConnectorResponse = { authorization_url: string };
  export type BudgetScopeDto = "organization" | "agent";
  export type BudgetResponse = {
    agent_id?: (null | AgentId) | undefined;
    agent_name?: (string | null) | undefined;
    alert: boolean;
    alert_threshold_percent: number;
    exceeded: boolean;
    monthly_limit_cents: number;
    percent: number;
    scope: BudgetScopeDto;
    spent_micros: number;
    updated_at: string;
    updated_by: string;
  };
  export type BudgetsResponse = {
    items: Array<BudgetResponse>;
    month_end: string;
    month_start: string;
    spent_micros: number;
  };
  export type CapabilitiesResponse = { chat: boolean; embeddings: boolean; ocr: boolean; transcription: boolean };
  export type CardColumnDto = "proposed" | "todo" | "in_progress" | "done" | "rejected";
  export type CardCommentId = string;
  export type CriterionDto = { done: boolean; text: string };
  export type ColumnDto = "proposed" | "todo" | "in_progress" | "done" | "rejected";
  export type EstimateDto = "xs" | "s" | "m" | "l" | "xl";
  export type PriorityDto = "low" | "normal" | "high" | "urgent";
  export type ProvenanceResponse = { end: number; quote: string; start: number; transcript_id: TranscriptId };
  export type CardResponse = {
    acceptance_criteria: Array<CriterionDto>;
    assignee_agent_id?: (null | AgentId) | undefined;
    assignee_subject?: (string | null) | undefined;
    client?: (null | ClientRefResponse) | undefined;
    column: ColumnDto;
    created_at: string;
    created_by_agent?: (string | null) | undefined;
    created_by_run?: (null | RunId) | undefined;
    created_by_subject: string;
    description: string;
    due_date?: (string | null) | undefined;
    estimate?: (null | EstimateDto) | undefined;
    id: CardId;
    labels: Array<string>;
    parent_id?: (null | CardId) | undefined;
    priority: PriorityDto;
    project?: (null | ProjectRefResponse) | undefined;
    provenance?: (null | ProvenanceResponse) | undefined;
    repository_hint?: (string | null) | undefined;
    title: string;
    updated_at: string;
    version: number;
  };
  export type CommentResponse = {
    author_actor?: (string | null) | undefined;
    author_subject: string;
    body: string;
    created_at: string;
    id: CardCommentId;
  };
  export type CardDetailResponse = { card: CardResponse; comments: Array<CommentResponse>; runs: Array<RunResponse> };
  export type CardListResponse = { items: Array<CardResponse> };
  export type ChatToolCallResponse = {
    arguments: string;
    latency_ms: number;
    name: string;
    succeeded: boolean;
    summary: string;
  };
  export type InstanceFailureResponse = { code: string; message: string };
  export type IntegrationEnvironmentDto = "production" | "staging" | "development";
  export type IntegrationInstanceId = string;
  export type IntegrationKindDto = "odoo_json2" | "odoo_jsonrpc" | "n8n";
  export type IntegrationInstanceResponse = {
    archived_at?: (string | null) | undefined;
    base_url: string;
    checked_at?: (string | null) | undefined;
    client_id: ClientId;
    created_at: string;
    created_by: string;
    database?: (string | null) | undefined;
    environment: IntegrationEnvironmentDto;
    id: IntegrationInstanceId;
    kind: IntegrationKindDto;
    login?: (string | null) | undefined;
    name: string;
    secret_preview: string;
    server_version?: (string | null) | undefined;
    updated_at: string;
  };
  export type CheckIntegrationInstanceResponse = {
    error?: (null | InstanceFailureResponse) | undefined;
    instance: IntegrationInstanceResponse;
    ok: boolean;
    server_version?: (string | null) | undefined;
    warning?: (string | null) | undefined;
  };
  export type ColumnCountResponse = { cards: number; column: ColumnDto };
  export type ClientResponse = {
    aliases: Array<string>;
    archived: boolean;
    created_at: string;
    created_by: string;
    id: ClientId;
    name: string;
    notes: string;
    updated_at: string;
  };
  export type ProjectResponse = {
    aliases: Array<string>;
    archived: boolean;
    client?: (null | ClientRefResponse) | undefined;
    created_at: string;
    created_by: string;
    description: string;
    id: ProjectId;
    name: string;
    repository?: (string | null) | undefined;
    updated_at: string;
  };
  export type ClientDetailResponse = {
    cards: Array<ColumnCountResponse>;
    client: ClientResponse;
    projects: Array<ProjectResponse>;
  };
  export type ClientListItemResponse = { client: ClientResponse; open_cards: number; project_count: number };
  export type ClientListResponse = { items: Array<ClientListItemResponse> };
  export type CommentCardRequest = { body: string };
  export type ConnectGithubRequest = { code: string; installation_id: number; state: string };
  export type ConversationId = string;
  export type ConversationSummaryResponse = {
    archived: boolean;
    cost_micros: number;
    created_at: string;
    full: boolean;
    id: ConversationId;
    input_tokens: number;
    model?: (string | null) | undefined;
    output_tokens: number;
    title: string;
    updated_at: string;
  };
  export type ConversationListResponse = { conversations: Array<ConversationSummaryResponse> };
  export type ProposedCardResponse = { card_id: string; title: string };
  export type MessageRoleDto = "user" | "assistant" | "tool";
  export type MessageStatusDto = "complete" | "interrupted" | "failed";
  export type MessageResponse = {
    cards: Array<ProposedCardResponse>;
    created_at: string;
    id: string;
    role: MessageRoleDto;
    seq: number;
    status: MessageStatusDto;
    text: string;
    tool_calls: Array<ChatToolCallResponse>;
  };
  export type ConversationResponse = { conversation: ConversationSummaryResponse; messages: Array<MessageResponse> };
  export type CostDimensionParam = "agent" | "user" | "model" | "day";
  export type CostRowResponse = {
    cost_micros: number;
    input_tokens: number;
    key: string;
    latency_ms_p50?: (number | null) | undefined;
    output_tokens: number;
    runs: number;
  };
  export type CostTotals = { cost_micros: number; input_tokens: number; output_tokens: number };
  export type CostsResponse = { rows: Array<CostRowResponse>; total: CostTotals };
  export type CreateCardRequest = {
    assignee_agent_id?: (null | AgentId) | undefined;
    client_id?: (null | ClientId) | undefined;
    description?: (string | null) | undefined;
    parent_id?: (null | CardId) | undefined;
    priority?: (null | PriorityDto) | undefined;
    project_id?: (null | ProjectId) | undefined;
    title: string;
  };
  export type CreateClientRequest = {
    aliases?: (Array<string> | null) | undefined;
    name: string;
    notes?: (string | null) | undefined;
  };
  export type CreateConversationRequest = { message: string };
  export type CreateGithubAppRequest = { code: string; state: string };
  export type CreateIntegrationInstanceRequest = {
    base_url: string;
    database?: (string | null) | undefined;
    environment: IntegrationEnvironmentDto;
    kind: IntegrationKindDto;
    login?: (string | null) | undefined;
    name: string;
    secret: string;
  };
  export type ManifestOwner = { type: "user" } | { login: string; type: "organization" };
  export type CreateManifestRequest = { owner: ManifestOwner };
  export type CreateMcpConnectorRequest = {
    name?: (string | null) | undefined;
    slug?: (string | null) | undefined;
    url: string;
  };
  export type CreateProjectRequest = {
    aliases?: (Array<string> | null) | undefined;
    client_id?: (null | ClientId) | undefined;
    description?: (string | null) | undefined;
    name: string;
    repository?: (string | null) | undefined;
  };
  export type IfMissedDto = "skip" | "run_once" | "run_all";
  export type IfOpenDto = "skip" | "queue" | "always";
  export type CreateRoutineRequest = {
    agent_id: AgentId;
    allow_writes?: boolean | undefined;
    enabled?: (boolean | null) | undefined;
    if_missed?: (null | IfMissedDto) | undefined;
    if_open?: (null | IfOpenDto) | undefined;
    name: string;
    priority?: (null | PriorityDto) | undefined;
    project_id?: (null | ProjectId) | undefined;
    prompt: string;
    schedule?: (null | ScheduleDto) | undefined;
    slug?: (string | null) | undefined;
    timezone?: (string | null) | undefined;
    title_template?: (string | null) | undefined;
  };
  export type CreateSkillRequest = {
    content?: (string | null) | undefined;
    description?: (string | null) | undefined;
    name: string;
    slug: string;
    source?: (string | null) | undefined;
  };
  export type CreateSourceRequest = {
    allow_writes?: boolean | undefined;
    auto_run?: boolean | undefined;
    config?: (Record<string, unknown> | null) | undefined;
    default_agent_id?: (null | AgentId) | undefined;
    description?: (string | null) | undefined;
    enabled?: (boolean | null) | undefined;
    kind?: (string | null) | undefined;
    name: string;
    project_id?: (null | ProjectId) | undefined;
    slug: string;
  };
  export type CreateTeamAgentRequest = {
    connector_grants?: (Array<string> | null) | undefined;
    description?: (string | null) | undefined;
    hue?: (number | null) | undefined;
    instructions?: (string | null) | undefined;
    model?: (string | null) | undefined;
    name: string;
    reports_to?: (null | AgentId) | undefined;
    repository_grants?: (Array<string> | null) | undefined;
    role?: (string | null) | undefined;
    slug: string;
  };
  export type CreateTranscriptionRequest = { file: string };
  export type CreatedConversationResponse = { id: ConversationId };
  export type DecideApprovalRequest = { decision: ApprovalDecisionDto; note?: (string | null) | undefined };
  export type DecideApprovalResponse = ApprovalResponse & Partial<{ resume_error: string | null }>;
  export type DecisionResponse = {
    action: string;
    actor?: (string | null) | undefined;
    allowed: boolean;
    decided_at: string;
    reason?: (string | null) | undefined;
    resource_id?: (string | null) | undefined;
    resource_kind: string;
  };
  export type DocumentTextResponse = { model: string; pages: number; text: string };
  export type ExecuteCardRequest = { expected_version: number; repository: string };
  export type ExecuteCardResponse = { card: CardResponse; run_id: RunId };
  export type ExecutionResponse = Partial<{
    branch: string | null;
    dropped_tool_calls: number;
    error: string | null;
    no_changes: boolean;
    pull_request: string | null;
    sha: string | null;
    summary: string | null;
  }>;
  export type GithubAppResponse = {
    created_at: string;
    created_by: string;
    html_url: string;
    owner_login: string;
    owner_type: string;
    slug: string;
  };
  export type GithubInstallationResponse = {
    account_login: string;
    connected_at: string;
    connected_by: string;
    installation_id: number;
  };
  export type GithubIntegrationResponse = Partial<{
    app: null | GithubAppResponse;
    installation: null | GithubInstallationResponse;
  }>;
  export type GrantAccessDto = "read" | "write";
  export type SourceEventId = string;
  export type HookResponse = { event_id: SourceEventId };
  export type InboxItemId = string;
  export type InboxKindDto = "approval" | "run" | "card" | "source" | "routine";
  export type InboxItemResponse = {
    body: string;
    card_id?: (null | CardId) | undefined;
    created_at: string;
    id: InboxItemId;
    kind: InboxKindDto;
    link?: (string | null) | undefined;
    read_at?: (string | null) | undefined;
    run_id?: (null | RunId) | undefined;
    title: string;
    unread: boolean;
  };
  export type InboxResponse = {
    items: Array<InboxItemResponse>;
    next_cursor?: (null | InboxItemId) | undefined;
    total: number;
    unread: number;
  };
  export type InstallUrlResponse = { url: string };
  export type InstanceGrantRequest = { access: GrantAccessDto; instance_id: IntegrationInstanceId };
  export type IntegrationActionId = string;
  export type IntegrationActionResponse = {
    access: ActionAccessDto;
    actor: string;
    after?: unknown | undefined;
    arguments: unknown;
    before?: unknown | undefined;
    card_id?: (null | CardId) | undefined;
    created_at: string;
    finished_at?: (string | null) | undefined;
    id: IntegrationActionId;
    instance_environment: IntegrationEnvironmentDto;
    instance_id: IntegrationInstanceId;
    instance_kind: IntegrationKindDto;
    instance_name: string;
    latency_ms?: (number | null) | undefined;
    outcome: ActionOutcomeDto;
    reason?: (string | null) | undefined;
    reason_code?: (string | null) | undefined;
    reverted_by_action_id?: (null | IntegrationActionId) | undefined;
    revertible: boolean;
    reverts_action_id?: (null | IntegrationActionId) | undefined;
    run_id?: (null | RunId) | undefined;
    target: unknown;
    tool: string;
  };
  export type IntegrationActionListResponse = { items: Array<IntegrationActionResponse> };
  export type IntegrationInstanceListResponse = { items: Array<IntegrationInstanceResponse> };
  export type InterveneCardRequest = {
    confirm_production_writes?: boolean | undefined;
    expected_version: number;
    grants: Array<InstanceGrantRequest>;
    instructions?: (string | null) | undefined;
  };
  export type InterveneCardResponse = { card: CardResponse; run_id: RunId };
  export type IssueResponse = {
    card_id?: (null | CardId) | undefined;
    html_url: string;
    labels: Array<string>;
    number: number;
    title: string;
  };
  export type IssueListResponse = { issues: Array<IssueResponse> };
  export type SourceId = string;
  export type SourceKindDto = "webhook";
  export type SourceResponse = {
    allow_writes: boolean;
    archived: boolean;
    auto_run: boolean;
    config: Record<string, unknown>;
    created_at: string;
    created_by: string;
    default_agent_id?: (null | AgentId) | undefined;
    description: string;
    enabled: boolean;
    events_last_24h: number;
    id: SourceId;
    kind: SourceKindDto;
    last_event_at?: (string | null) | undefined;
    name: string;
    project_id?: (null | ProjectId) | undefined;
    slug: string;
    updated_at: string;
  };
  export type IssuedSourceResponse = { source: SourceResponse; token: string; webhook_url: string };
  export type RunCountsResponse = { created: number; failed: number; skipped: number };
  export type RoutineId = string;
  export type RoutineRunId = string;
  export type RunOriginDto = "planning" | "manual" | "webhook";
  export type RoutineRunStatusDto = "pending" | "queued" | "created" | "skipped" | "failed";
  export type RoutineTriggerId = string;
  export type RoutineRunResponse = {
    card_column?: (null | CardColumnDto) | undefined;
    card_id?: (null | CardId) | undefined;
    id: RoutineRunId;
    origin: RunOriginDto;
    reason?: (string | null) | undefined;
    routine_id: RoutineId;
    run_finished_at?: (string | null) | undefined;
    run_id?: (null | RunId) | undefined;
    run_started_at?: (string | null) | undefined;
    run_status?: (null | AgentRunStatusDto) | undefined;
    scheduled_for: string;
    started_at: string;
    status: RoutineRunStatusDto;
    trigger_id?: (null | RoutineTriggerId) | undefined;
  };
  export type TriggerResponse = {
    enabled: boolean;
    id: RoutineTriggerId;
    kind: TriggerKindDto;
    next_run_at?: (string | null) | undefined;
    schedule?: (null | ScheduleDto) | undefined;
    token_hint?: (string | null) | undefined;
  };
  export type RoutineResponse = {
    agent_id: AgentId;
    allow_writes: boolean;
    archived: boolean;
    counts: RunCountsResponse;
    created_at: string;
    created_by: string;
    enabled: boolean;
    id: RoutineId;
    if_missed: IfMissedDto;
    if_open: IfOpenDto;
    last_run?: (null | RoutineRunResponse) | undefined;
    last_run_at?: (string | null) | undefined;
    name: string;
    next_run_at?: (string | null) | undefined;
    priority: PriorityDto;
    project_id?: (null | ProjectId) | undefined;
    prompt: string;
    slug: string;
    timezone: string;
    title_template: string;
    triggers: Array<TriggerResponse>;
    updated_at: string;
  };
  export type IssuedTriggerResponse = {
    routine: RoutineResponse;
    token?: (string | null) | undefined;
    trigger_id: RoutineTriggerId;
    webhook_url?: (string | null) | undefined;
  };
  export type LaunchWorksitesRequest = {
    issues?: Array<number> | undefined;
    prompts?: Array<string> | undefined;
    repository: string;
  };
  export type LaunchedWorksiteResponse = { card: CardResponse; run_id: RunId };
  export type LaunchWorksitesResponse = { worksites: Array<LaunchedWorksiteResponse> };
  export type LimitRequest = { alert_threshold_percent?: (number | null) | undefined; monthly_limit_cents: number };
  export type ManifestResponse = { action_url: string; manifest: string; state: string };
  export type MarkReadRequest = Partial<{ all: boolean; ids: Array<InboxItemId> | null }>;
  export type MarkReadResponse = { unread: number; updated: number };
  export type McpConnectorId = string;
  export type McpConnectorResponse = {
    archived: boolean;
    connected: boolean;
    created_at: string;
    created_by: string;
    expires_at?: (string | null) | undefined;
    id: McpConnectorId;
    issuer: string;
    name: string;
    resource: string;
    scopes: Array<string>;
    slug: string;
    updated_at: string;
    url: string;
  };
  export type McpConnectorListResponse = { items: Array<McpConnectorResponse> };
  export type McpOAuthCallbackRequest = { code: string; state: string };
  export type McpOAuthCallbackResponse = { connector_id: McpConnectorId };
  export type McpToolResponse = { name: string; read_only: boolean };
  export type OrganizationId = string;
  export type MeOrganization = { id: OrganizationId; name: string; slug: string };
  export type MeResponse = {
    display_name?: (string | null) | undefined;
    email?: (string | null) | undefined;
    organization: MeOrganization;
    subject: string;
    username?: (string | null) | undefined;
  };
  export type MemberResponse = {
    email?: (string | null) | undefined;
    last_seen_at: string;
    name: string;
    subject: string;
  };
  export type MemberListResponse = { items: Array<MemberResponse> };
  export type ProviderKindDto = "anthropic" | "mistral" | "openai_compatible";
  export type ModelRefDto = { model: string; provider: ProviderKindDto };
  export type ModelAssignmentsDto = {
    captain?: (null | ModelRefDto) | undefined;
    embeddings: ModelRefDto;
    executor?: (string | null) | undefined;
    extractor: ModelRefDto;
    integrator?: (null | ModelRefDto) | undefined;
    judge: ModelRefDto;
    reader?: (null | ModelRefDto) | undefined;
    transcriber?: (null | ModelRefDto) | undefined;
    writer: ModelRefDto;
  };
  export type ModelAssignmentsResponse = Partial<{ assignments: null | ModelAssignmentsDto }>;
  export type ProviderErrorResponse = { message: string; provider: ProviderKindDto };
  export type PriceResponse = Partial<{ input_per_mtok_micros: number | null; output_per_mtok_micros: number | null }>;
  export type ModelResponse = {
    capabilities: CapabilitiesResponse;
    context_window?: (number | null) | undefined;
    display_name: string;
    id: string;
    price?: (null | PriceResponse) | undefined;
    provider: ProviderKindDto;
  };
  export type ModelCatalogResponse = { errors: Array<ProviderErrorResponse>; models: Array<ModelResponse> };
  export type ModelSpendResponse = { cost_micros: number; input_tokens: number; model: string; output_tokens: number };
  export type MoveCardRequest = { expected_version: number; to: ColumnDto };
  export type PatchCardRequest = {
    acceptance_criteria?: (Array<CriterionDto> | null) | undefined;
    assignee_agent_id?: (null | AgentId) | undefined;
    assignee_subject?: (string | null) | undefined;
    client_id?: (null | ClientId) | undefined;
    description?: (string | null) | undefined;
    due_date?: (string | null) | undefined;
    estimate?: (null | EstimateDto) | undefined;
    expected_version: number;
    labels?: (Array<string> | null) | undefined;
    parent_id?: (null | CardId) | undefined;
    priority?: (null | PriorityDto) | undefined;
    project_id?: (null | ProjectId) | undefined;
    title?: (string | null) | undefined;
  };
  export type ProjectSpendResponse = {
    agents: Array<AgentSpendResponse>;
    cost_micros: number;
    input_tokens: number;
    name?: (string | null) | undefined;
    output_tokens: number;
    project_id?: (null | ProjectId) | undefined;
  };
  export type ProjectCostsResponse = {
    from: string;
    rows: Array<ProjectSpendResponse>;
    to: string;
    total: SpentResponse;
  };
  export type ProjectDetailResponse = { cards: Array<ColumnCountResponse>; project: ProjectResponse };
  export type ProjectListItemResponse = { open_cards: number; project: ProjectResponse };
  export type ProjectListResponse = { items: Array<ProjectListItemResponse> };
  export type ProviderResponse = {
    base_url?: (string | null) | undefined;
    created_by: string;
    kind: ProviderKindDto;
    preview: string;
    updated_at: string;
  };
  export type ProviderListResponse = { providers: Array<ProviderResponse> };
  export type PutProviderRequest = Partial<{ api_key: string; base_url: string | null }>;
  export type PutUserProviderRequest = {
    api_key?: (string | null) | undefined;
    base_url?: (string | null) | undefined;
    default_model: string;
    is_default?: (boolean | null) | undefined;
  };
  export type ReadDocumentRequest = { file: string };
  export type RepositoryResponse = {
    full_name: string;
    html_url: string;
    name: string;
    owner: string;
    private: boolean;
  };
  export type RepositoryListResponse = { repositories: Array<RepositoryResponse> };
  export type RevertIntegrationActionRequest = Partial<{ confirm_production: boolean }>;
  export type RoutineListResponse = { items: Array<RoutineResponse> };
  export type RoutineRunListResponse = { items: Array<RoutineRunResponse>; total: number };
  export type RunAgentRequest = {
    allow_writes?: boolean | undefined;
    base_branch?: (string | null) | undefined;
    expected_version: number;
    repository?: (string | null) | undefined;
  };
  export type RunAgentResponse = { run_id: RunId };
  export type RunListResponse = { items: Array<RunResponse>; total: number };
  export type StepStatusResponse = "running" | "succeeded" | "failed" | "skipped";
  export type ToolCallResponse = { latency_ms: number; succeeded: boolean; summary: string; tool: string };
  export type StepResponse = {
    agent?: (string | null) | undefined;
    cost_micros: number;
    detail?: (string | null) | undefined;
    error?: (string | null) | undefined;
    execution?: (null | ExecutionResponse) | undefined;
    finished_at?: (string | null) | undefined;
    input_tokens: number;
    kind: string;
    latency_ms?: (number | null) | undefined;
    model?: (string | null) | undefined;
    output_tokens: number;
    seq: number;
    started_at: string;
    status: StepStatusResponse;
    tool_calls: Array<ToolCallResponse>;
  };
  export type UsageTotals = { cost_micros: number; input_tokens: number; output_tokens: number };
  export type RunTimelineResponse = {
    decisions: Array<DecisionResponse>;
    run: RunResponse;
    steps: Array<StepResponse>;
    totals: UsageTotals;
  };
  export type SaveBudgetsRequest = Partial<{ agents: Array<AgentLimitRequest>; organization: null | LimitRequest }>;
  export type SendMessageRequest = Partial<{ message: string | null }>;
  export type SkillId = string;
  export type SkillResponse = {
    archived: boolean;
    content: string;
    created_at: string;
    created_by: string;
    description: string;
    id: SkillId;
    name: string;
    slug: string;
    source: string;
    updated_at: string;
    used_by: number;
  };
  export type SkillListResponse = { items: Array<SkillResponse> };
  export type SourceEventStatusDto = "received" | "card_created" | "ignored" | "failed";
  export type SourceEventResponse = {
    card_id?: (null | CardId) | undefined;
    error?: (string | null) | undefined;
    id: SourceEventId;
    received_at: string;
    run_id?: (null | RunId) | undefined;
    source_id: SourceId;
    status: SourceEventStatusDto;
    summary: string;
  };
  export type SourceEventListResponse = { items: Array<SourceEventResponse>; total: number };
  export type SourceListResponse = { items: Array<SourceResponse> };
  export type SubmitTranscriptRequest = {
    body: string;
    client_id?: (null | ClientId) | undefined;
    meeting_date?: (string | null) | undefined;
    participants?: (Array<string> | null) | undefined;
    project_id?: (null | ProjectId) | undefined;
    repository?: (string | null) | undefined;
    title: string;
  };
  export type SubmitTranscriptResponse = { run_id: RunId; transcript_id: TranscriptId };
  export type TaskSpendResponse = {
    agents: Array<AgentSpendResponse>;
    card_id: CardId;
    cost_micros: number;
    input_tokens: number;
    output_tokens: number;
    project_id?: (null | ProjectId) | undefined;
    title: string;
  };
  export type TaskCostsResponse = { from: string; rows: Array<TaskSpendResponse>; to: string; total: SpentResponse };
  export type TeamAgentStatusDto = "idle" | "running";
  export type TeamAgentResponse = {
    archived: boolean;
    config: Record<string, unknown>;
    connector_grants: Array<string>;
    created_at: string;
    created_by: string;
    description: string;
    hue: number;
    id: AgentId;
    instructions: string;
    model: string;
    name: string;
    paused: boolean;
    reports_to?: (null | AgentId) | undefined;
    repository_grants: Array<string>;
    revision: number;
    role: string;
    skills: Array<string>;
    slug: string;
    status: TeamAgentStatusDto;
    updated_at: string;
  };
  export type TeamAgentListResponse = { items: Array<TeamAgentResponse> };
  export type TeamAgentRevisionResponse = { author: string; changed: string; created_at: string; revision: number };
  export type TeamAgentRevisionListResponse = { items: Array<TeamAgentRevisionResponse> };
  export type TestMcpConnectorResponse = { tools: Array<McpToolResponse> };
  export type TestProviderResponse = { error?: (string | null) | undefined; models: Array<ModelResponse>; ok: boolean };
  export type TestUserProviderResponse = { error?: (string | null) | undefined; ok: boolean };
  export type TranscriptionResponse = { model: string; text: string };
  export type UpdateClientRequest = Partial<{
    aliases: Array<string> | null;
    archived: boolean | null;
    name: string | null;
    notes: string | null;
  }>;
  export type UpdateConversationRequest = Partial<{ archived: boolean | null; title: string | null }>;
  export type UpdateIntegrationInstanceRequest = Partial<{
    base_url: string | null;
    database: string | null;
    environment: null | IntegrationEnvironmentDto;
    login: string | null;
    name: string | null;
    secret: string | null;
  }>;
  export type UpdateMcpConnectorRequest = Partial<{ archived: boolean | null; name: string | null }>;
  export type UpdateProjectRequest = Partial<{
    aliases: Array<string> | null;
    archived: boolean | null;
    client_id: null | ClientId;
    description: string | null;
    name: string | null;
    repository: string | null;
  }>;
  export type UpdateRoutineRequest = Partial<{
    agent_id: null | AgentId;
    allow_writes: boolean | null;
    archived: boolean | null;
    enabled: boolean | null;
    if_missed: null | IfMissedDto;
    if_open: null | IfOpenDto;
    name: string | null;
    priority: null | PriorityDto;
    project_id: null | ProjectId;
    prompt: string | null;
    slug: string | null;
    timezone: string | null;
    title_template: string | null;
  }>;
  export type UpdateSkillRequest = Partial<{
    archived: boolean | null;
    content: string | null;
    description: string | null;
    name: string | null;
    slug: string | null;
    source: string | null;
  }>;
  export type UpdateSourceRequest = Partial<{
    allow_writes: boolean | null;
    archived: boolean | null;
    auto_run: boolean | null;
    config: Record<string, unknown> | null;
    default_agent_id: null | AgentId;
    description: string | null;
    enabled: boolean | null;
    name: string | null;
    project_id: null | ProjectId;
    slug: string | null;
  }>;
  export type UpdateTeamAgentRequest = Partial<{
    archived: boolean | null;
    change_summary: string | null;
    config: Record<string, unknown> | null;
    connector_grants: Array<string> | null;
    description: string | null;
    hue: number | null;
    instructions: string | null;
    model: string | null;
    name: string | null;
    paused: boolean | null;
    reports_to: null | AgentId;
    repository_grants: Array<string> | null;
    role: string | null;
    skills: Array<string> | null;
    slug: string | null;
  }>;
  export type UpdateTriggerRequest = Partial<{ enabled: boolean | null; schedule: null | ScheduleDto }>;
  export type UserSpendResponse = {
    cost_micros: number;
    input_tokens: number;
    models: Array<ModelSpendResponse>;
    output_tokens: number;
    user: string;
  };
  export type UserCostsResponse = { from: string; rows: Array<UserSpendResponse>; to: string; total: SpentResponse };
  export type UserProviderResponse = {
    base_url?: (string | null) | undefined;
    default_model: string;
    is_default: boolean;
    kind: ProviderKindDto;
    preview: string;
    updated_at: string;
  };
  export type WorksiteResponse = {
    card_id: CardId;
    column: string;
    created_at: string;
    issue_number?: (number | null) | undefined;
    issue_url?: (string | null) | undefined;
    repository?: (string | null) | undefined;
    run_id?: (null | RunId) | undefined;
    run_status?: (string | null) | undefined;
    title: string;
  };
  export type WorksiteListResponse = { items: Array<WorksiteResponse> };

  // </Schemas>
}

export namespace Endpoints {
  // <Endpoints>

  export type get_ListAgents = {
    method: "GET";
    path: "/api/v1/agents";
    requestFormat: "json";
    parameters: never;
    responses: { 200: Array<Schemas.AgentResponse>; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type get_GetCosts = {
    method: "GET";
    path: "/api/v1/analytics/costs";
    requestFormat: "json";
    parameters: {
      query: Partial<{ by: "agent" | "user" | "model" | "day"; from: string; to: string }>;
    };
    responses: {
      200: Schemas.CostsResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListAgentCosts = {
    method: "GET";
    path: "/api/v1/analytics/costs/agents";
    requestFormat: "json";
    parameters: {
      query: Partial<{ from: string; to: string; agent_id: string; card_id: string }>;
    };
    responses: {
      200: Schemas.AgentCostsResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListProjectCosts = {
    method: "GET";
    path: "/api/v1/analytics/costs/projects";
    requestFormat: "json";
    parameters: {
      query: Partial<{ from: string; to: string; agent_id: string; card_id: string }>;
    };
    responses: {
      200: Schemas.ProjectCostsResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListTaskCosts = {
    method: "GET";
    path: "/api/v1/analytics/costs/tasks";
    requestFormat: "json";
    parameters: {
      query: Partial<{ from: string; to: string; agent_id: string; card_id: string; project_id: string }>;
    };
    responses: {
      200: Schemas.TaskCostsResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListUserCosts = {
    method: "GET";
    path: "/api/v1/analytics/costs/users";
    requestFormat: "json";
    parameters: {
      query: Partial<{ from: string; to: string; agent_id: string; card_id: string }>;
    };
    responses: {
      200: Schemas.UserCostsResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListApprovals = {
    method: "GET";
    path: "/api/v1/approvals";
    requestFormat: "json";
    parameters: {
      query: Partial<{ status: "pending" | "approved" | "rejected" | "expired" }>;
    };
    responses: {
      200: Schemas.ApprovalListResponse;
      400: Schemas.ApiErrorResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetApproval = {
    method: "GET";
    path: "/api/v1/approvals/{approval_id}";
    requestFormat: "json";
    parameters: {
      path: { approval_id: string };
    };
    responses: {
      200: Schemas.ApprovalResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type post_DecideApproval = {
    method: "POST";
    path: "/api/v1/approvals/{approval_id}/decide";
    requestFormat: "json";
    parameters: {
      path: { approval_id: string };

      body: Schemas.DecideApprovalRequest;
    };
    responses: {
      200: Schemas.DecideApprovalResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListArtifacts = {
    method: "GET";
    path: "/api/v1/artifacts";
    requestFormat: "json";
    parameters: {
      query: Partial<{
        card_id: string;
        agent_id: string;
        kind: "markdown" | "json" | "link";
        limit: number;
        cursor: string;
      }>;
    };
    responses: {
      200: Schemas.ArtifactListResponse;
      400: Schemas.ApiErrorResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetArtifact = {
    method: "GET";
    path: "/api/v1/artifacts/{artifact_id}";
    requestFormat: "json";
    parameters: {
      path: { artifact_id: string };
    };
    responses: {
      200: Schemas.ArtifactResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListAuditEvents = {
    method: "GET";
    path: "/api/v1/audit/events";
    requestFormat: "json";
    parameters: {
      query: Partial<{
        topic: string;
        type: string;
        actor: string;
        agent_id: string;
        card_id: string;
        from: string;
        to: string;
        limit: number;
        cursor: string;
      }>;
    };
    responses: {
      200: Schemas.AuditEventListResponse;
      400: Schemas.ApiErrorResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListBudgets = {
    method: "GET";
    path: "/api/v1/budgets";
    requestFormat: "json";
    parameters: never;
    responses: { 200: Schemas.BudgetsResponse; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type put_SaveBudgets = {
    method: "PUT";
    path: "/api/v1/budgets";
    requestFormat: "json";
    parameters: {
      body: Schemas.SaveBudgetsRequest;
    };
    responses: {
      200: Schemas.BudgetsResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListCards = {
    method: "GET";
    path: "/api/v1/cards";
    requestFormat: "json";
    parameters: {
      query: Partial<{
        client_id: string;
        project_id: string;
        priority: string;
        assignee: string;
        label: string;
        column: string;
        status: string;
        assignee_agent_id: string;
      }>;
    };
    responses: { 200: Schemas.CardListResponse; 401: Schemas.ApiErrorResponse; 422: Schemas.ApiErrorResponse };
  };
  export type post_CreateCard = {
    method: "POST";
    path: "/api/v1/cards";
    requestFormat: "json";
    parameters: {
      body: Schemas.CreateCardRequest;
    };
    responses: {
      201: Schemas.CardResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetCard = {
    method: "GET";
    path: "/api/v1/cards/{card_id}";
    requestFormat: "json";
    parameters: {
      path: { card_id: string };
    };
    responses: { 200: Schemas.CardDetailResponse; 401: Schemas.ApiErrorResponse; 404: Schemas.ApiErrorResponse };
  };
  export type patch_PatchCard = {
    method: "PATCH";
    path: "/api/v1/cards/{card_id}";
    requestFormat: "json";
    parameters: {
      path: { card_id: string };

      body: Schemas.PatchCardRequest;
    };
    responses: {
      200: Schemas.CardResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_RunCardAgent = {
    method: "POST";
    path: "/api/v1/cards/{card_id}/agent-runs";
    requestFormat: "json";
    parameters: {
      path: { card_id: string };

      body: Schemas.RunAgentRequest;
    };
    responses: {
      202: Schemas.RunAgentResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_CommentCard = {
    method: "POST";
    path: "/api/v1/cards/{card_id}/comments";
    requestFormat: "json";
    parameters: {
      path: { card_id: string };

      body: Schemas.CommentCardRequest;
    };
    responses: {
      201: Schemas.CommentResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_ExecuteCard = {
    method: "POST";
    path: "/api/v1/cards/{card_id}/executions";
    requestFormat: "json";
    parameters: {
      path: { card_id: string };

      body: Schemas.ExecuteCardRequest;
    };
    responses: {
      202: Schemas.ExecuteCardResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListCardIntegrationActions = {
    method: "GET";
    path: "/api/v1/cards/{card_id}/integration-actions";
    requestFormat: "json";
    parameters: {
      path: { card_id: string };
    };
    responses: {
      200: Schemas.IntegrationActionListResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type post_InterveneCard = {
    method: "POST";
    path: "/api/v1/cards/{card_id}/interventions";
    requestFormat: "json";
    parameters: {
      path: { card_id: string };

      body: Schemas.InterveneCardRequest;
    };
    responses: {
      202: Schemas.InterveneCardResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_MoveCard = {
    method: "POST";
    path: "/api/v1/cards/{card_id}/move";
    requestFormat: "json";
    parameters: {
      path: { card_id: string };

      body: Schemas.MoveCardRequest;
    };
    responses: {
      200: Schemas.CardResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListClients = {
    method: "GET";
    path: "/api/v1/clients";
    requestFormat: "json";
    parameters: {
      query: Partial<{ q: string; archived: boolean }>;
    };
    responses: { 200: Schemas.ClientListResponse; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type post_CreateClient = {
    method: "POST";
    path: "/api/v1/clients";
    requestFormat: "json";
    parameters: {
      body: Schemas.CreateClientRequest;
    };
    responses: {
      201: Schemas.ClientResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetClient = {
    method: "GET";
    path: "/api/v1/clients/{client_id}";
    requestFormat: "json";
    parameters: {
      path: { client_id: string };
    };
    responses: {
      200: Schemas.ClientDetailResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type patch_UpdateClient = {
    method: "PATCH";
    path: "/api/v1/clients/{client_id}";
    requestFormat: "json";
    parameters: {
      path: { client_id: string };

      body: Schemas.UpdateClientRequest;
    };
    responses: {
      200: Schemas.ClientResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListIntegrationInstances = {
    method: "GET";
    path: "/api/v1/clients/{client_id}/integrations";
    requestFormat: "json";
    parameters: {
      query: Partial<{ archived: boolean }>;
      path: { client_id: string };
    };
    responses: {
      200: Schemas.IntegrationInstanceListResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type post_CreateIntegrationInstance = {
    method: "POST";
    path: "/api/v1/clients/{client_id}/integrations";
    requestFormat: "json";
    parameters: {
      path: { client_id: string };

      body: Schemas.CreateIntegrationInstanceRequest;
    };
    responses: {
      201: Schemas.IntegrationInstanceResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListConversations = {
    method: "GET";
    path: "/api/v1/conversations";
    requestFormat: "json";
    parameters: never;
    responses: { 200: Schemas.ConversationListResponse; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type post_CreateConversation = {
    method: "POST";
    path: "/api/v1/conversations";
    requestFormat: "json";
    parameters: {
      body: Schemas.CreateConversationRequest;
    };
    responses: {
      201: Schemas.CreatedConversationResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetConversation = {
    method: "GET";
    path: "/api/v1/conversations/{id}";
    requestFormat: "json";
    parameters: {
      path: { id: string };
    };
    responses: {
      200: Schemas.ConversationResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type patch_UpdateConversation = {
    method: "PATCH";
    path: "/api/v1/conversations/{id}";
    requestFormat: "json";
    parameters: {
      path: { id: string };

      body: Schemas.UpdateConversationRequest;
    };
    responses: {
      200: Schemas.ConversationSummaryResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_SendConversationMessage = {
    method: "POST";
    path: "/api/v1/conversations/{id}/messages";
    requestFormat: "json";
    parameters: {
      path: { id: string };

      body: Schemas.SendMessageRequest;
    };
    responses: {
      200: unknown;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_ReadDocument = {
    method: "POST";
    path: "/api/v1/documents/text";
    requestFormat: "form-data";
    parameters: {
      body: Schemas.ReadDocumentRequest;
    };
    responses: {
      200: Schemas.DocumentTextResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      413: Schemas.ApiErrorResponse;
      415: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
      502: Schemas.ApiErrorResponse;
      503: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListInbox = {
    method: "GET";
    path: "/api/v1/inbox";
    requestFormat: "json";
    parameters: {
      query: Partial<{ unread_only: boolean; limit: number; cursor: string }>;
    };
    responses: {
      200: Schemas.InboxResponse;
      400: Schemas.ApiErrorResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_MarkInboxRead = {
    method: "POST";
    path: "/api/v1/inbox/read";
    requestFormat: "json";
    parameters: {
      body: Schemas.MarkReadRequest;
    };
    responses: {
      200: Schemas.MarkReadResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_RevertIntegrationAction = {
    method: "POST";
    path: "/api/v1/integration-actions/{id}/revert";
    requestFormat: "json";
    parameters: {
      path: { id: string };

      body: Schemas.RevertIntegrationActionRequest;
    };
    responses: {
      200: Schemas.IntegrationActionListResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type delete_ArchiveIntegrationInstance = {
    method: "DELETE";
    path: "/api/v1/integration-instances/{id}";
    requestFormat: "json";
    parameters: {
      path: { id: string };
    };
    responses: {
      200: Schemas.IntegrationInstanceResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
    };
  };
  export type patch_UpdateIntegrationInstance = {
    method: "PATCH";
    path: "/api/v1/integration-instances/{id}";
    requestFormat: "json";
    parameters: {
      path: { id: string };

      body: Schemas.UpdateIntegrationInstanceRequest;
    };
    responses: {
      200: Schemas.IntegrationInstanceResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_CheckIntegrationInstance = {
    method: "POST";
    path: "/api/v1/integration-instances/{id}/check";
    requestFormat: "json";
    parameters: {
      path: { id: string };
    };
    responses: {
      200: Schemas.CheckIntegrationInstanceResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetGithubIntegration = {
    method: "GET";
    path: "/api/v1/integrations/github";
    requestFormat: "json";
    parameters: never;
    responses: { 200: Schemas.GithubIntegrationResponse; 401: Schemas.ApiErrorResponse };
  };
  export type delete_DisconnectGithub = {
    method: "DELETE";
    path: "/api/v1/integrations/github";
    requestFormat: "json";
    parameters: never;
    responses: { 200: Schemas.GithubIntegrationResponse; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type post_CreateGithubApp = {
    method: "POST";
    path: "/api/v1/integrations/github/app";
    requestFormat: "json";
    parameters: {
      body: Schemas.CreateGithubAppRequest;
    };
    responses: {
      201: Schemas.GithubAppResponse;
      400: Schemas.ApiErrorResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
      503: Schemas.ApiErrorResponse;
    };
  };
  export type delete_DeleteGithubApp = {
    method: "DELETE";
    path: "/api/v1/integrations/github/app";
    requestFormat: "json";
    parameters: never;
    responses: {
      200: Schemas.GithubAppResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type post_CreateGithubAppManifest = {
    method: "POST";
    path: "/api/v1/integrations/github/app/manifest";
    requestFormat: "json";
    parameters: {
      body: Schemas.CreateManifestRequest;
    };
    responses: {
      200: Schemas.ManifestResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
      503: Schemas.ApiErrorResponse;
    };
  };
  export type post_ConnectGithub = {
    method: "POST";
    path: "/api/v1/integrations/github/callback";
    requestFormat: "json";
    parameters: {
      body: Schemas.ConnectGithubRequest;
    };
    responses: {
      200: Schemas.GithubIntegrationResponse;
      400: Schemas.ApiErrorResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      503: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetGithubInstallUrl = {
    method: "GET";
    path: "/api/v1/integrations/github/install";
    requestFormat: "json";
    parameters: never;
    responses: {
      200: Schemas.InstallUrlResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      503: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListGithubIssues = {
    method: "GET";
    path: "/api/v1/integrations/github/issues";
    requestFormat: "json";
    parameters: {
      query: { repository: string };
    };
    responses: {
      200: Schemas.IssueListResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListGithubRepositories = {
    method: "GET";
    path: "/api/v1/integrations/github/repositories";
    requestFormat: "json";
    parameters: never;
    responses: { 200: Schemas.RepositoryListResponse; 401: Schemas.ApiErrorResponse; 409: Schemas.ApiErrorResponse };
  };
  export type get_ListMcpConnectors = {
    method: "GET";
    path: "/api/v1/mcp-connectors";
    requestFormat: "json";
    parameters: {
      query: Partial<{ archived: boolean }>;
    };
    responses: { 200: Schemas.McpConnectorListResponse; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type post_CreateMcpConnector = {
    method: "POST";
    path: "/api/v1/mcp-connectors";
    requestFormat: "json";
    parameters: {
      body: Schemas.CreateMcpConnectorRequest;
    };
    responses: {
      201: Schemas.McpConnectorResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
      503: Schemas.ApiErrorResponse;
    };
  };
  export type post_CompleteMcpOAuth = {
    method: "POST";
    path: "/api/v1/mcp-connectors/oauth/callback";
    requestFormat: "json";
    parameters: {
      body: Schemas.McpOAuthCallbackRequest;
    };
    responses: {
      200: Schemas.McpOAuthCallbackResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
      503: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetMcpConnector = {
    method: "GET";
    path: "/api/v1/mcp-connectors/{connector_id}";
    requestFormat: "json";
    parameters: {
      path: { connector_id: string };
    };
    responses: {
      200: Schemas.McpConnectorResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type patch_UpdateMcpConnector = {
    method: "PATCH";
    path: "/api/v1/mcp-connectors/{connector_id}";
    requestFormat: "json";
    parameters: {
      path: { connector_id: string };

      body: Schemas.UpdateMcpConnectorRequest;
    };
    responses: {
      200: Schemas.McpConnectorResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_AuthorizeMcpConnector = {
    method: "POST";
    path: "/api/v1/mcp-connectors/{connector_id}/authorize";
    requestFormat: "json";
    parameters: {
      path: { connector_id: string };
    };
    responses: {
      200: Schemas.AuthorizeMcpConnectorResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      503: Schemas.ApiErrorResponse;
    };
  };
  export type delete_DisconnectMcpConnector = {
    method: "DELETE";
    path: "/api/v1/mcp-connectors/{connector_id}/connection";
    requestFormat: "json";
    parameters: {
      path: { connector_id: string };
    };
    responses: {
      204: unknown;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type post_TestMcpConnector = {
    method: "POST";
    path: "/api/v1/mcp-connectors/{connector_id}/test";
    requestFormat: "json";
    parameters: {
      path: { connector_id: string };
    };
    responses: {
      200: Schemas.TestMcpConnectorResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
      503: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetMe = {
    method: "GET";
    path: "/api/v1/me";
    requestFormat: "json";
    parameters: never;
    responses: { 200: Schemas.MeResponse; 401: Schemas.ApiErrorResponse };
  };
  export type get_ListMyProviders = {
    method: "GET";
    path: "/api/v1/me/providers";
    requestFormat: "json";
    parameters: never;
    responses: {
      200: Array<Schemas.UserProviderResponse>;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
    };
  };
  export type put_PutMyProvider = {
    method: "PUT";
    path: "/api/v1/me/providers/{kind}";
    requestFormat: "json";
    parameters: {
      path: { kind: "anthropic" | "mistral" | "openai_compatible" };

      body: Schemas.PutUserProviderRequest;
    };
    responses: {
      200: Schemas.UserProviderResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type delete_DeleteMyProvider = {
    method: "DELETE";
    path: "/api/v1/me/providers/{kind}";
    requestFormat: "json";
    parameters: {
      path: { kind: "anthropic" | "mistral" | "openai_compatible" };
    };
    responses: {
      204: unknown;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type post_TestMyProvider = {
    method: "POST";
    path: "/api/v1/me/providers/{kind}/test";
    requestFormat: "json";
    parameters: {
      path: { kind: "anthropic" | "mistral" | "openai_compatible" };
    };
    responses: {
      200: Schemas.TestUserProviderResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListMembers = {
    method: "GET";
    path: "/api/v1/members";
    requestFormat: "json";
    parameters: never;
    responses: { 200: Schemas.MemberListResponse; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type get_GetModelAssignments = {
    method: "GET";
    path: "/api/v1/model-assignments";
    requestFormat: "json";
    parameters: never;
    responses: { 200: Schemas.ModelAssignmentsResponse; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type put_PutModelAssignments = {
    method: "PUT";
    path: "/api/v1/model-assignments";
    requestFormat: "json";
    parameters: {
      body: Schemas.ModelAssignmentsDto;
    };
    responses: {
      200: Schemas.ModelAssignmentsResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListModels = {
    method: "GET";
    path: "/api/v1/models";
    requestFormat: "json";
    parameters: never;
    responses: { 200: Schemas.ModelCatalogResponse; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type get_ListProjects = {
    method: "GET";
    path: "/api/v1/projects";
    requestFormat: "json";
    parameters: {
      query: Partial<{ q: string; archived: boolean; client_id: string }>;
    };
    responses: { 200: Schemas.ProjectListResponse; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type post_CreateProject = {
    method: "POST";
    path: "/api/v1/projects";
    requestFormat: "json";
    parameters: {
      body: Schemas.CreateProjectRequest;
    };
    responses: {
      201: Schemas.ProjectResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetProject = {
    method: "GET";
    path: "/api/v1/projects/{project_id}";
    requestFormat: "json";
    parameters: {
      path: { project_id: string };
    };
    responses: {
      200: Schemas.ProjectDetailResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type patch_UpdateProject = {
    method: "PATCH";
    path: "/api/v1/projects/{project_id}";
    requestFormat: "json";
    parameters: {
      path: { project_id: string };

      body: Schemas.UpdateProjectRequest;
    };
    responses: {
      200: Schemas.ProjectResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListWorksites = {
    method: "GET";
    path: "/api/v1/projects/{project_id}/worksites";
    requestFormat: "json";
    parameters: {
      path: { project_id: string };
    };
    responses: { 200: Schemas.WorksiteListResponse; 401: Schemas.ApiErrorResponse; 404: Schemas.ApiErrorResponse };
  };
  export type post_LaunchWorksites = {
    method: "POST";
    path: "/api/v1/projects/{project_id}/worksites";
    requestFormat: "json";
    parameters: {
      path: { project_id: string };

      body: Schemas.LaunchWorksitesRequest;
    };
    responses: {
      202: Schemas.LaunchWorksitesResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListProviders = {
    method: "GET";
    path: "/api/v1/providers";
    requestFormat: "json";
    parameters: never;
    responses: { 200: Schemas.ProviderListResponse; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type put_PutProvider = {
    method: "PUT";
    path: "/api/v1/providers/{kind}";
    requestFormat: "json";
    parameters: {
      path: { kind: "anthropic" | "mistral" | "openai_compatible" };

      body: Schemas.PutProviderRequest;
    };
    responses: {
      200: Schemas.ProviderResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type delete_DeleteProvider = {
    method: "DELETE";
    path: "/api/v1/providers/{kind}";
    requestFormat: "json";
    parameters: {
      path: { kind: "anthropic" | "mistral" | "openai_compatible" };
    };
    responses: {
      204: unknown;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type post_TestProvider = {
    method: "POST";
    path: "/api/v1/providers/{kind}/test";
    requestFormat: "json";
    parameters: {
      path: { kind: "anthropic" | "mistral" | "openai_compatible" };
    };
    responses: {
      200: Schemas.TestProviderResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListRoutines = {
    method: "GET";
    path: "/api/v1/routines";
    requestFormat: "json";
    parameters: {
      query: Partial<{ archived: boolean }>;
    };
    responses: { 200: Schemas.RoutineListResponse; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type post_CreateRoutine = {
    method: "POST";
    path: "/api/v1/routines";
    requestFormat: "json";
    parameters: {
      body: Schemas.CreateRoutineRequest;
    };
    responses: {
      201: Schemas.RoutineResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetRoutine = {
    method: "GET";
    path: "/api/v1/routines/{routine_id}";
    requestFormat: "json";
    parameters: {
      path: { routine_id: string };
    };
    responses: {
      200: Schemas.RoutineResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type patch_UpdateRoutine = {
    method: "PATCH";
    path: "/api/v1/routines/{routine_id}";
    requestFormat: "json";
    parameters: {
      path: { routine_id: string };

      body: Schemas.UpdateRoutineRequest;
    };
    responses: {
      200: Schemas.RoutineResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_ArchiveRoutine = {
    method: "POST";
    path: "/api/v1/routines/{routine_id}/archive";
    requestFormat: "json";
    parameters: {
      path: { routine_id: string };
    };
    responses: {
      200: Schemas.RoutineResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type post_LaunchRoutine = {
    method: "POST";
    path: "/api/v1/routines/{routine_id}/launch";
    requestFormat: "json";
    parameters: {
      path: { routine_id: string };
    };
    responses: {
      202: Schemas.RoutineRunResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListRoutineRuns = {
    method: "GET";
    path: "/api/v1/routines/{routine_id}/runs";
    requestFormat: "json";
    parameters: {
      query: Partial<{ limit: number; offset: number }>;
      path: { routine_id: string };
    };
    responses: {
      200: Schemas.RoutineRunListResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type post_AddRoutineTrigger = {
    method: "POST";
    path: "/api/v1/routines/{routine_id}/triggers";
    requestFormat: "json";
    parameters: {
      path: { routine_id: string };

      body: Schemas.AddTriggerRequest;
    };
    responses: {
      201: Schemas.IssuedTriggerResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type delete_RemoveRoutineTrigger = {
    method: "DELETE";
    path: "/api/v1/routines/{routine_id}/triggers/{trigger_id}";
    requestFormat: "json";
    parameters: {
      path: { routine_id: string; trigger_id: string };
    };
    responses: {
      200: Schemas.RoutineResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type patch_UpdateRoutineTrigger = {
    method: "PATCH";
    path: "/api/v1/routines/{routine_id}/triggers/{trigger_id}";
    requestFormat: "json";
    parameters: {
      path: { routine_id: string; trigger_id: string };

      body: Schemas.UpdateTriggerRequest;
    };
    responses: {
      200: Schemas.RoutineResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_RotateRoutineTriggerToken = {
    method: "POST";
    path: "/api/v1/routines/{routine_id}/triggers/{trigger_id}/rotate-token";
    requestFormat: "json";
    parameters: {
      path: { routine_id: string; trigger_id: string };
    };
    responses: {
      200: Schemas.IssuedTriggerResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListRuns = {
    method: "GET";
    path: "/api/v1/runs";
    requestFormat: "json";
    parameters: {
      query: Partial<{
        limit: number;
        offset: number;
        status: "queued" | "running" | "succeeded" | "failed" | "cancelled";
        agent_id: string;
      }>;
    };
    responses: { 200: Schemas.RunListResponse; 400: Schemas.ApiErrorResponse; 401: Schemas.ApiErrorResponse };
  };
  export type get_ListActiveRuns = {
    method: "GET";
    path: "/api/v1/runs/active";
    requestFormat: "json";
    parameters: never;
    responses: { 200: Schemas.ActiveRunListResponse; 401: Schemas.ApiErrorResponse };
  };
  export type get_GetRun = {
    method: "GET";
    path: "/api/v1/runs/{run_id}";
    requestFormat: "json";
    parameters: {
      path: { run_id: string };
    };
    responses: { 200: Schemas.RunResponse; 401: Schemas.ApiErrorResponse; 404: Schemas.ApiErrorResponse };
  };
  export type get_ListRunIntegrationActions = {
    method: "GET";
    path: "/api/v1/runs/{run_id}/integration-actions";
    requestFormat: "json";
    parameters: {
      path: { run_id: string };
    };
    responses: {
      200: Schemas.IntegrationActionListResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetRunTimeline = {
    method: "GET";
    path: "/api/v1/runs/{run_id}/timeline";
    requestFormat: "json";
    parameters: {
      path: { run_id: string };
    };
    responses: { 200: Schemas.RunTimelineResponse; 401: Schemas.ApiErrorResponse; 404: Schemas.ApiErrorResponse };
  };
  export type get_ListSkills = {
    method: "GET";
    path: "/api/v1/skills";
    requestFormat: "json";
    parameters: {
      query: Partial<{ archived: boolean }>;
    };
    responses: { 200: Schemas.SkillListResponse; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type post_CreateSkill = {
    method: "POST";
    path: "/api/v1/skills";
    requestFormat: "json";
    parameters: {
      body: Schemas.CreateSkillRequest;
    };
    responses: {
      201: Schemas.SkillResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetSkill = {
    method: "GET";
    path: "/api/v1/skills/{skill_id}";
    requestFormat: "json";
    parameters: {
      path: { skill_id: string };
    };
    responses: {
      200: Schemas.SkillResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type patch_UpdateSkill = {
    method: "PATCH";
    path: "/api/v1/skills/{skill_id}";
    requestFormat: "json";
    parameters: {
      path: { skill_id: string };

      body: Schemas.UpdateSkillRequest;
    };
    responses: {
      200: Schemas.SkillResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_ArchiveSkill = {
    method: "POST";
    path: "/api/v1/skills/{skill_id}/archive";
    requestFormat: "json";
    parameters: {
      path: { skill_id: string };
    };
    responses: {
      200: Schemas.SkillResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListSources = {
    method: "GET";
    path: "/api/v1/sources";
    requestFormat: "json";
    parameters: {
      query: Partial<{ archived: boolean }>;
    };
    responses: { 200: Schemas.SourceListResponse; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type post_CreateSource = {
    method: "POST";
    path: "/api/v1/sources";
    requestFormat: "json";
    parameters: {
      body: Schemas.CreateSourceRequest;
    };
    responses: {
      201: Schemas.IssuedSourceResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetSource = {
    method: "GET";
    path: "/api/v1/sources/{source_id}";
    requestFormat: "json";
    parameters: {
      path: { source_id: string };
    };
    responses: {
      200: Schemas.SourceResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type patch_UpdateSource = {
    method: "PATCH";
    path: "/api/v1/sources/{source_id}";
    requestFormat: "json";
    parameters: {
      path: { source_id: string };

      body: Schemas.UpdateSourceRequest;
    };
    responses: {
      200: Schemas.SourceResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_ArchiveSource = {
    method: "POST";
    path: "/api/v1/sources/{source_id}/archive";
    requestFormat: "json";
    parameters: {
      path: { source_id: string };
    };
    responses: {
      200: Schemas.SourceResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListSourceEvents = {
    method: "GET";
    path: "/api/v1/sources/{source_id}/events";
    requestFormat: "json";
    parameters: {
      query: Partial<{ limit: number; offset: number }>;
      path: { source_id: string };
    };
    responses: {
      200: Schemas.SourceEventListResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type post_RotateSourceToken = {
    method: "POST";
    path: "/api/v1/sources/{source_id}/rotate-token";
    requestFormat: "json";
    parameters: {
      path: { source_id: string };
    };
    responses: {
      200: Schemas.IssuedSourceResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type post_TestSource = {
    method: "POST";
    path: "/api/v1/sources/{source_id}/test";
    requestFormat: "json";
    parameters: {
      path: { source_id: string };
    };
    responses: {
      202: Schemas.SourceEventResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
      429: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListTeamAgents = {
    method: "GET";
    path: "/api/v1/team/agents";
    requestFormat: "json";
    parameters: {
      query: Partial<{ archived: boolean }>;
    };
    responses: { 200: Schemas.TeamAgentListResponse; 401: Schemas.ApiErrorResponse; 403: Schemas.ApiErrorResponse };
  };
  export type post_CreateTeamAgent = {
    method: "POST";
    path: "/api/v1/team/agents";
    requestFormat: "json";
    parameters: {
      body: Schemas.CreateTeamAgentRequest;
    };
    responses: {
      201: Schemas.TeamAgentResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetTeamAgent = {
    method: "GET";
    path: "/api/v1/team/agents/{agent_id}";
    requestFormat: "json";
    parameters: {
      path: { agent_id: string };
    };
    responses: {
      200: Schemas.TeamAgentResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type patch_UpdateTeamAgent = {
    method: "PATCH";
    path: "/api/v1/team/agents/{agent_id}";
    requestFormat: "json";
    parameters: {
      path: { agent_id: string };

      body: Schemas.UpdateTeamAgentRequest;
    };
    responses: {
      200: Schemas.TeamAgentResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_ListTeamAgentRevisions = {
    method: "GET";
    path: "/api/v1/team/agents/{agent_id}/revisions";
    requestFormat: "json";
    parameters: {
      path: { agent_id: string };
    };
    responses: {
      200: Schemas.TeamAgentRevisionListResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
    };
  };
  export type post_RestoreTeamAgentRevision = {
    method: "POST";
    path: "/api/v1/team/agents/{agent_id}/revisions/{revision}/restore";
    requestFormat: "json";
    parameters: {
      path: { agent_id: string; revision: number };
    };
    responses: {
      200: Schemas.TeamAgentResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      404: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type post_CreateTranscription = {
    method: "POST";
    path: "/api/v1/transcriptions";
    requestFormat: "form-data";
    parameters: {
      body: Schemas.CreateTranscriptionRequest;
    };
    responses: {
      200: Schemas.TranscriptionResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      409: Schemas.ApiErrorResponse;
      413: Schemas.ApiErrorResponse;
      415: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
      502: Schemas.ApiErrorResponse;
      503: Schemas.ApiErrorResponse;
    };
  };
  export type post_SubmitTranscript = {
    method: "POST";
    path: "/api/v1/transcripts";
    requestFormat: "json";
    parameters: {
      body: Schemas.SubmitTranscriptRequest;
    };
    responses: {
      202: Schemas.SubmitTranscriptResponse;
      401: Schemas.ApiErrorResponse;
      403: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
    };
  };
  export type get_GetLiveness = {
    method: "GET";
    path: "/health/live";
    requestFormat: "json";
    parameters: never;
    responses: { 204: unknown };
  };
  export type get_GetReadiness = {
    method: "GET";
    path: "/health/ready";
    requestFormat: "json";
    parameters: never;
    responses: { 204: unknown; 503: Schemas.ApiErrorResponse };
  };
  export type post_TriggerRoutine = {
    method: "POST";
    path: "/hooks/routines/{token}";
    requestFormat: "json";
    parameters: {
      path: { token: string };
    };
    responses: { 202: Schemas.RoutineRunResponse; 404: Schemas.ApiErrorResponse };
  };
  export type post_ReceiveSourceEvent = {
    method: "POST";
    path: "/hooks/sources/{token}";
    requestFormat: "json";
    parameters: {
      path: { token: string };

      body: Record<string, unknown>;
    };
    responses: {
      202: Schemas.HookResponse;
      404: Schemas.ApiErrorResponse;
      413: Schemas.ApiErrorResponse;
      422: Schemas.ApiErrorResponse;
      429: Schemas.ApiErrorResponse;
    };
  };

  // </Endpoints>
}

// <EndpointByMethod>
export type EndpointByMethod = {
  get: {
    "/api/v1/agents": Endpoints.get_ListAgents;
    "/api/v1/analytics/costs": Endpoints.get_GetCosts;
    "/api/v1/analytics/costs/agents": Endpoints.get_ListAgentCosts;
    "/api/v1/analytics/costs/projects": Endpoints.get_ListProjectCosts;
    "/api/v1/analytics/costs/tasks": Endpoints.get_ListTaskCosts;
    "/api/v1/analytics/costs/users": Endpoints.get_ListUserCosts;
    "/api/v1/approvals": Endpoints.get_ListApprovals;
    "/api/v1/approvals/{approval_id}": Endpoints.get_GetApproval;
    "/api/v1/artifacts": Endpoints.get_ListArtifacts;
    "/api/v1/artifacts/{artifact_id}": Endpoints.get_GetArtifact;
    "/api/v1/audit/events": Endpoints.get_ListAuditEvents;
    "/api/v1/budgets": Endpoints.get_ListBudgets;
    "/api/v1/cards": Endpoints.get_ListCards;
    "/api/v1/cards/{card_id}": Endpoints.get_GetCard;
    "/api/v1/cards/{card_id}/integration-actions": Endpoints.get_ListCardIntegrationActions;
    "/api/v1/clients": Endpoints.get_ListClients;
    "/api/v1/clients/{client_id}": Endpoints.get_GetClient;
    "/api/v1/clients/{client_id}/integrations": Endpoints.get_ListIntegrationInstances;
    "/api/v1/conversations": Endpoints.get_ListConversations;
    "/api/v1/conversations/{id}": Endpoints.get_GetConversation;
    "/api/v1/inbox": Endpoints.get_ListInbox;
    "/api/v1/integrations/github": Endpoints.get_GetGithubIntegration;
    "/api/v1/integrations/github/install": Endpoints.get_GetGithubInstallUrl;
    "/api/v1/integrations/github/issues": Endpoints.get_ListGithubIssues;
    "/api/v1/integrations/github/repositories": Endpoints.get_ListGithubRepositories;
    "/api/v1/mcp-connectors": Endpoints.get_ListMcpConnectors;
    "/api/v1/mcp-connectors/{connector_id}": Endpoints.get_GetMcpConnector;
    "/api/v1/me": Endpoints.get_GetMe;
    "/api/v1/me/providers": Endpoints.get_ListMyProviders;
    "/api/v1/members": Endpoints.get_ListMembers;
    "/api/v1/model-assignments": Endpoints.get_GetModelAssignments;
    "/api/v1/models": Endpoints.get_ListModels;
    "/api/v1/projects": Endpoints.get_ListProjects;
    "/api/v1/projects/{project_id}": Endpoints.get_GetProject;
    "/api/v1/projects/{project_id}/worksites": Endpoints.get_ListWorksites;
    "/api/v1/providers": Endpoints.get_ListProviders;
    "/api/v1/routines": Endpoints.get_ListRoutines;
    "/api/v1/routines/{routine_id}": Endpoints.get_GetRoutine;
    "/api/v1/routines/{routine_id}/runs": Endpoints.get_ListRoutineRuns;
    "/api/v1/runs": Endpoints.get_ListRuns;
    "/api/v1/runs/active": Endpoints.get_ListActiveRuns;
    "/api/v1/runs/{run_id}": Endpoints.get_GetRun;
    "/api/v1/runs/{run_id}/integration-actions": Endpoints.get_ListRunIntegrationActions;
    "/api/v1/runs/{run_id}/timeline": Endpoints.get_GetRunTimeline;
    "/api/v1/skills": Endpoints.get_ListSkills;
    "/api/v1/skills/{skill_id}": Endpoints.get_GetSkill;
    "/api/v1/sources": Endpoints.get_ListSources;
    "/api/v1/sources/{source_id}": Endpoints.get_GetSource;
    "/api/v1/sources/{source_id}/events": Endpoints.get_ListSourceEvents;
    "/api/v1/team/agents": Endpoints.get_ListTeamAgents;
    "/api/v1/team/agents/{agent_id}": Endpoints.get_GetTeamAgent;
    "/api/v1/team/agents/{agent_id}/revisions": Endpoints.get_ListTeamAgentRevisions;
    "/health/live": Endpoints.get_GetLiveness;
    "/health/ready": Endpoints.get_GetReadiness;
  };
  post: {
    "/api/v1/approvals/{approval_id}/decide": Endpoints.post_DecideApproval;
    "/api/v1/cards": Endpoints.post_CreateCard;
    "/api/v1/cards/{card_id}/agent-runs": Endpoints.post_RunCardAgent;
    "/api/v1/cards/{card_id}/comments": Endpoints.post_CommentCard;
    "/api/v1/cards/{card_id}/executions": Endpoints.post_ExecuteCard;
    "/api/v1/cards/{card_id}/interventions": Endpoints.post_InterveneCard;
    "/api/v1/cards/{card_id}/move": Endpoints.post_MoveCard;
    "/api/v1/clients": Endpoints.post_CreateClient;
    "/api/v1/clients/{client_id}/integrations": Endpoints.post_CreateIntegrationInstance;
    "/api/v1/conversations": Endpoints.post_CreateConversation;
    "/api/v1/conversations/{id}/messages": Endpoints.post_SendConversationMessage;
    "/api/v1/documents/text": Endpoints.post_ReadDocument;
    "/api/v1/inbox/read": Endpoints.post_MarkInboxRead;
    "/api/v1/integration-actions/{id}/revert": Endpoints.post_RevertIntegrationAction;
    "/api/v1/integration-instances/{id}/check": Endpoints.post_CheckIntegrationInstance;
    "/api/v1/integrations/github/app": Endpoints.post_CreateGithubApp;
    "/api/v1/integrations/github/app/manifest": Endpoints.post_CreateGithubAppManifest;
    "/api/v1/integrations/github/callback": Endpoints.post_ConnectGithub;
    "/api/v1/mcp-connectors": Endpoints.post_CreateMcpConnector;
    "/api/v1/mcp-connectors/oauth/callback": Endpoints.post_CompleteMcpOAuth;
    "/api/v1/mcp-connectors/{connector_id}/authorize": Endpoints.post_AuthorizeMcpConnector;
    "/api/v1/mcp-connectors/{connector_id}/test": Endpoints.post_TestMcpConnector;
    "/api/v1/me/providers/{kind}/test": Endpoints.post_TestMyProvider;
    "/api/v1/projects": Endpoints.post_CreateProject;
    "/api/v1/projects/{project_id}/worksites": Endpoints.post_LaunchWorksites;
    "/api/v1/providers/{kind}/test": Endpoints.post_TestProvider;
    "/api/v1/routines": Endpoints.post_CreateRoutine;
    "/api/v1/routines/{routine_id}/archive": Endpoints.post_ArchiveRoutine;
    "/api/v1/routines/{routine_id}/launch": Endpoints.post_LaunchRoutine;
    "/api/v1/routines/{routine_id}/triggers": Endpoints.post_AddRoutineTrigger;
    "/api/v1/routines/{routine_id}/triggers/{trigger_id}/rotate-token": Endpoints.post_RotateRoutineTriggerToken;
    "/api/v1/skills": Endpoints.post_CreateSkill;
    "/api/v1/skills/{skill_id}/archive": Endpoints.post_ArchiveSkill;
    "/api/v1/sources": Endpoints.post_CreateSource;
    "/api/v1/sources/{source_id}/archive": Endpoints.post_ArchiveSource;
    "/api/v1/sources/{source_id}/rotate-token": Endpoints.post_RotateSourceToken;
    "/api/v1/sources/{source_id}/test": Endpoints.post_TestSource;
    "/api/v1/team/agents": Endpoints.post_CreateTeamAgent;
    "/api/v1/team/agents/{agent_id}/revisions/{revision}/restore": Endpoints.post_RestoreTeamAgentRevision;
    "/api/v1/transcriptions": Endpoints.post_CreateTranscription;
    "/api/v1/transcripts": Endpoints.post_SubmitTranscript;
    "/hooks/routines/{token}": Endpoints.post_TriggerRoutine;
    "/hooks/sources/{token}": Endpoints.post_ReceiveSourceEvent;
  };
  put: {
    "/api/v1/budgets": Endpoints.put_SaveBudgets;
    "/api/v1/me/providers/{kind}": Endpoints.put_PutMyProvider;
    "/api/v1/model-assignments": Endpoints.put_PutModelAssignments;
    "/api/v1/providers/{kind}": Endpoints.put_PutProvider;
  };
  patch: {
    "/api/v1/cards/{card_id}": Endpoints.patch_PatchCard;
    "/api/v1/clients/{client_id}": Endpoints.patch_UpdateClient;
    "/api/v1/conversations/{id}": Endpoints.patch_UpdateConversation;
    "/api/v1/integration-instances/{id}": Endpoints.patch_UpdateIntegrationInstance;
    "/api/v1/mcp-connectors/{connector_id}": Endpoints.patch_UpdateMcpConnector;
    "/api/v1/projects/{project_id}": Endpoints.patch_UpdateProject;
    "/api/v1/routines/{routine_id}": Endpoints.patch_UpdateRoutine;
    "/api/v1/routines/{routine_id}/triggers/{trigger_id}": Endpoints.patch_UpdateRoutineTrigger;
    "/api/v1/skills/{skill_id}": Endpoints.patch_UpdateSkill;
    "/api/v1/sources/{source_id}": Endpoints.patch_UpdateSource;
    "/api/v1/team/agents/{agent_id}": Endpoints.patch_UpdateTeamAgent;
  };
  delete: {
    "/api/v1/integration-instances/{id}": Endpoints.delete_ArchiveIntegrationInstance;
    "/api/v1/integrations/github": Endpoints.delete_DisconnectGithub;
    "/api/v1/integrations/github/app": Endpoints.delete_DeleteGithubApp;
    "/api/v1/mcp-connectors/{connector_id}/connection": Endpoints.delete_DisconnectMcpConnector;
    "/api/v1/me/providers/{kind}": Endpoints.delete_DeleteMyProvider;
    "/api/v1/providers/{kind}": Endpoints.delete_DeleteProvider;
    "/api/v1/routines/{routine_id}/triggers/{trigger_id}": Endpoints.delete_RemoveRoutineTrigger;
  };
};

// </EndpointByMethod>

// <EndpointByMethod.Shorthands>
export type GetEndpoints = EndpointByMethod["get"];
export type PostEndpoints = EndpointByMethod["post"];
export type PutEndpoints = EndpointByMethod["put"];
export type PatchEndpoints = EndpointByMethod["patch"];
export type DeleteEndpoints = EndpointByMethod["delete"];
// </EndpointByMethod.Shorthands>

// <ApiClientTypes>
export type EndpointParameters = {
  body?: unknown;
  query?: Record<string, unknown>;
  header?: Record<string, unknown>;
  path?: Record<string, unknown>;
};

export type MutationMethod = "post" | "put" | "patch" | "delete";
export type Method = "get" | "head" | "options" | MutationMethod;

type RequestFormat = "json" | "form-data" | "form-url" | "binary" | "text";

export type DefaultEndpoint = {
  parameters?: EndpointParameters | undefined;
  responses?: Record<string, unknown>;
  responseHeaders?: Record<string, unknown>;
};

export type Endpoint<TConfig extends DefaultEndpoint = DefaultEndpoint> = {
  operationId: string;
  method: Method;
  path: string;
  requestFormat: RequestFormat;
  parameters?: TConfig["parameters"];
  meta: {
    alias: string;
    hasParameters: boolean;
    areParametersRequired: boolean;
  };
  responses?: TConfig["responses"];
  responseHeaders?: TConfig["responseHeaders"];
};

export interface Fetcher {
  decodePathParams?: (path: string, pathParams: Record<string, string>) => string;
  encodeSearchParams?: (searchParams: Record<string, unknown> | undefined) => URLSearchParams;
  //
  fetch: (input: {
    method: Method;
    url: URL;
    urlSearchParams?: URLSearchParams | undefined;
    parameters?: EndpointParameters | undefined;
    path: string;
    overrides?: RequestInit;
    throwOnStatusError?: boolean;
  }) => Promise<Response>;
  parseResponseData?: (response: Response) => Promise<unknown>;
}

export const successStatusCodes = [
  200, 201, 202, 203, 204, 205, 206, 207, 208, 226, 300, 301, 302, 303, 304, 305, 306, 307, 308,
] as const;
export type SuccessStatusCode = (typeof successStatusCodes)[number];

export const errorStatusCodes = [
  400, 401, 402, 403, 404, 405, 406, 407, 408, 409, 410, 411, 412, 413, 414, 415, 416, 417, 418, 421, 422, 423, 424,
  425, 426, 428, 429, 431, 451, 500, 501, 502, 503, 504, 505, 506, 507, 508, 510, 511,
] as const;
export type ErrorStatusCode = (typeof errorStatusCodes)[number];

// Taken from https://github.com/unjs/fetchdts/blob/ec4eaeab5d287116171fc1efd61f4a1ad34e4609/src/fetch.ts#L3
export interface TypedHeaders<TypedHeaderValues extends Record<string, string> | unknown>
  extends Omit<Headers, "append" | "delete" | "get" | "getSetCookie" | "has" | "set" | "forEach"> {
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/Headers/append) */
  append: <Name extends Extract<keyof TypedHeaderValues, string> | (string & {})>(
    name: Name,
    value: Lowercase<Name> extends keyof TypedHeaderValues ? TypedHeaderValues[Lowercase<Name>] : string,
  ) => void;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/Headers/delete) */
  delete: <Name extends Extract<keyof TypedHeaderValues, string> | (string & {})>(name: Name) => void;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/Headers/get) */
  get: <Name extends Extract<keyof TypedHeaderValues, string> | (string & {})>(
    name: Name,
  ) => (Lowercase<Name> extends keyof TypedHeaderValues ? TypedHeaderValues[Lowercase<Name>] : string) | null;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/Headers/getSetCookie) */
  getSetCookie: () => string[];
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/Headers/has) */
  has: <Name extends Extract<keyof TypedHeaderValues, string> | (string & {})>(name: Name) => boolean;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/Headers/set) */
  set: <Name extends Extract<keyof TypedHeaderValues, string> | (string & {})>(
    name: Name,
    value: Lowercase<Name> extends keyof TypedHeaderValues ? TypedHeaderValues[Lowercase<Name>] : string,
  ) => void;
  forEach: (
    callbackfn: (
      value: TypedHeaderValues[keyof TypedHeaderValues] | (string & {}),
      key: Extract<keyof TypedHeaderValues, string> | (string & {}),
      parent: TypedHeaders<TypedHeaderValues>,
    ) => void,
    thisArg?: any,
  ) => void;
}

/** @see https://developer.mozilla.org/en-US/docs/Web/API/Response */
export interface TypedSuccessResponse<TSuccess, TStatusCode, THeaders>
  extends Omit<Response, "ok" | "status" | "json" | "headers"> {
  ok: true;
  status: TStatusCode;
  headers: never extends THeaders ? Headers : TypedHeaders<THeaders>;
  data: TSuccess;
  /** [MDN Reference](https://developer.mozilla.org/en-US/docs/Web/API/Response/json) */
  json: () => Promise<TSuccess>;
}

/** @see https://developer.mozilla.org/en-US/docs/Web/API/Response */
export interface TypedErrorResponse<TData, TStatusCode, THeaders>
  extends Omit<Response, "ok" | "status" | "json" | "headers"> {
  ok: false;
  status: TStatusCode;
  headers: never extends THeaders ? Headers : TypedHeaders<THeaders>;
  data: TData;
  /** [MDN Reference](https://developer.mozilla.org/en-US/docs/Web/API/Response/json) */
  json: () => Promise<TData>;
}

export type TypedApiResponse<TAllResponses extends Record<string | number, unknown> = {}, THeaders = {}> = {
  [K in keyof TAllResponses]: K extends string
    ? K extends `${infer TStatusCode extends number}`
      ? TStatusCode extends SuccessStatusCode
        ? TypedSuccessResponse<TAllResponses[K], TStatusCode, K extends keyof THeaders ? THeaders[K] : never>
        : TypedErrorResponse<TAllResponses[K], TStatusCode, K extends keyof THeaders ? THeaders[K] : never>
      : never
    : K extends number
      ? K extends SuccessStatusCode
        ? TypedSuccessResponse<TAllResponses[K], K, K extends keyof THeaders ? THeaders[K] : never>
        : TypedErrorResponse<TAllResponses[K], K, K extends keyof THeaders ? THeaders[K] : never>
      : never;
}[keyof TAllResponses];

export type SafeApiResponse<TEndpoint> = TEndpoint extends { responses: infer TResponses }
  ? TResponses extends Record<string, unknown>
    ? TypedApiResponse<TResponses, TEndpoint extends { responseHeaders: infer THeaders } ? THeaders : never>
    : never
  : never;

export type InferResponseByStatus<TEndpoint, TStatusCode> = Extract<
  SafeApiResponse<TEndpoint>,
  { status: TStatusCode }
>;

type RequiredKeys<T> = {
  [P in keyof T]-?: undefined extends T[P] ? never : P;
}[keyof T];

type MaybeOptionalArg<T> = RequiredKeys<T> extends never ? [config?: T] : [config: T];
type NotNever<T> = [T] extends [never] ? false : true;

// </ApiClientTypes>

// <TypedStatusError>
export class TypedStatusError<TData = unknown> extends Error {
  response: TypedErrorResponse<TData, ErrorStatusCode, unknown>;
  status: number;
  constructor(response: TypedErrorResponse<TData, ErrorStatusCode, unknown>) {
    super(`HTTP ${response.status}: ${response.statusText}`);
    this.name = "TypedStatusError";
    this.response = response;
    this.status = response.status;
  }
}
// </TypedStatusError>

// <ApiClient>
export class ApiClient {
  baseUrl: string = "";
  successStatusCodes = successStatusCodes;
  errorStatusCodes = errorStatusCodes;

  constructor(public fetcher: Fetcher) {}

  setBaseUrl(baseUrl: string) {
    this.baseUrl = baseUrl;
    return this;
  }

  /**
   * Replace path parameters in URL
   * Supports both OpenAPI format {param} and Express format :param
   */
  defaultDecodePathParams = (url: string, params: Record<string, string>): string => {
    return url
      .replace(/{(\w+)}/g, (_, key: string) => params[key] || `{${key}}`)
      .replace(/:([a-zA-Z0-9_]+)/g, (_, key: string) => params[key] || `:${key}`);
  };

  /** Uses URLSearchParams, skips null/undefined values */
  defaultEncodeSearchParams = (queryParams: Record<string, unknown> | undefined): URLSearchParams | undefined => {
    if (!queryParams) return;

    const searchParams = new URLSearchParams();
    Object.entries(queryParams).forEach(([key, value]) => {
      if (value != null) {
        // Skip null/undefined values
        if (Array.isArray(value)) {
          value.forEach((val) => val != null && searchParams.append(key, String(val)));
        } else {
          searchParams.append(key, String(value));
        }
      }
    });

    return searchParams;
  };

  defaultParseResponseData = async (response: Response): Promise<unknown> => {
    const contentType = response.headers.get("content-type") ?? "";
    if (contentType.startsWith("text/")) {
      return await response.text();
    }

    if (contentType === "application/octet-stream") {
      return await response.arrayBuffer();
    }

    if (
      contentType.includes("application/json") ||
      (contentType.includes("application/") && contentType.includes("json")) ||
      contentType === "*/*"
    ) {
      try {
        return await response.json();
      } catch {
        return undefined;
      }
    }

    return;
  };

  // <ApiClient.get>
  get<Path extends keyof GetEndpoints, TEndpoint extends GetEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<
      TEndpoint extends { parameters: infer UParams }
        ? NotNever<UParams> extends true
          ? UParams & { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
          : { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
        : { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
    >
  ): Promise<Extract<InferResponseByStatus<TEndpoint, SuccessStatusCode>, { data: {} }>["data"]>;

  get<Path extends keyof GetEndpoints, TEndpoint extends GetEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<
      TEndpoint extends { parameters: infer UParams }
        ? NotNever<UParams> extends true
          ? UParams & { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
          : { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
        : { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
    >
  ): Promise<SafeApiResponse<TEndpoint>>;

  get<Path extends keyof GetEndpoints, _TEndpoint extends GetEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<any>
  ): Promise<any> {
    return this.request("get", path, ...params);
  }
  // </ApiClient.get>

  // <ApiClient.post>
  post<Path extends keyof PostEndpoints, TEndpoint extends PostEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<
      TEndpoint extends { parameters: infer UParams }
        ? NotNever<UParams> extends true
          ? UParams & { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
          : { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
        : { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
    >
  ): Promise<Extract<InferResponseByStatus<TEndpoint, SuccessStatusCode>, { data: {} }>["data"]>;

  post<Path extends keyof PostEndpoints, TEndpoint extends PostEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<
      TEndpoint extends { parameters: infer UParams }
        ? NotNever<UParams> extends true
          ? UParams & { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
          : { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
        : { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
    >
  ): Promise<SafeApiResponse<TEndpoint>>;

  post<Path extends keyof PostEndpoints, _TEndpoint extends PostEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<any>
  ): Promise<any> {
    return this.request("post", path, ...params);
  }
  // </ApiClient.post>

  // <ApiClient.put>
  put<Path extends keyof PutEndpoints, TEndpoint extends PutEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<
      TEndpoint extends { parameters: infer UParams }
        ? NotNever<UParams> extends true
          ? UParams & { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
          : { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
        : { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
    >
  ): Promise<Extract<InferResponseByStatus<TEndpoint, SuccessStatusCode>, { data: {} }>["data"]>;

  put<Path extends keyof PutEndpoints, TEndpoint extends PutEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<
      TEndpoint extends { parameters: infer UParams }
        ? NotNever<UParams> extends true
          ? UParams & { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
          : { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
        : { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
    >
  ): Promise<SafeApiResponse<TEndpoint>>;

  put<Path extends keyof PutEndpoints, _TEndpoint extends PutEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<any>
  ): Promise<any> {
    return this.request("put", path, ...params);
  }
  // </ApiClient.put>

  // <ApiClient.patch>
  patch<Path extends keyof PatchEndpoints, TEndpoint extends PatchEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<
      TEndpoint extends { parameters: infer UParams }
        ? NotNever<UParams> extends true
          ? UParams & { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
          : { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
        : { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
    >
  ): Promise<Extract<InferResponseByStatus<TEndpoint, SuccessStatusCode>, { data: {} }>["data"]>;

  patch<Path extends keyof PatchEndpoints, TEndpoint extends PatchEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<
      TEndpoint extends { parameters: infer UParams }
        ? NotNever<UParams> extends true
          ? UParams & { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
          : { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
        : { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
    >
  ): Promise<SafeApiResponse<TEndpoint>>;

  patch<Path extends keyof PatchEndpoints, _TEndpoint extends PatchEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<any>
  ): Promise<any> {
    return this.request("patch", path, ...params);
  }
  // </ApiClient.patch>

  // <ApiClient.delete>
  delete<Path extends keyof DeleteEndpoints, TEndpoint extends DeleteEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<
      TEndpoint extends { parameters: infer UParams }
        ? NotNever<UParams> extends true
          ? UParams & { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
          : { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
        : { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
    >
  ): Promise<Extract<InferResponseByStatus<TEndpoint, SuccessStatusCode>, { data: {} }>["data"]>;

  delete<Path extends keyof DeleteEndpoints, TEndpoint extends DeleteEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<
      TEndpoint extends { parameters: infer UParams }
        ? NotNever<UParams> extends true
          ? UParams & { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
          : { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
        : { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
    >
  ): Promise<SafeApiResponse<TEndpoint>>;

  delete<Path extends keyof DeleteEndpoints, _TEndpoint extends DeleteEndpoints[Path]>(
    path: Path,
    ...params: MaybeOptionalArg<any>
  ): Promise<any> {
    return this.request("delete", path, ...params);
  }
  // </ApiClient.delete>

  // <ApiClient.request>
  /**
   * Generic request method with full type-safety for any endpoint
   */
  request<
    TMethod extends keyof EndpointByMethod,
    TPath extends keyof EndpointByMethod[TMethod],
    TEndpoint extends EndpointByMethod[TMethod][TPath],
  >(
    method: TMethod,
    path: TPath,
    ...params: MaybeOptionalArg<
      TEndpoint extends { parameters: infer UParams }
        ? NotNever<UParams> extends true
          ? UParams & { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
          : { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
        : { overrides?: RequestInit; withResponse?: false; throwOnStatusError?: boolean }
    >
  ): Promise<Extract<InferResponseByStatus<TEndpoint, SuccessStatusCode>, { data: {} }>["data"]>;

  request<
    TMethod extends keyof EndpointByMethod,
    TPath extends keyof EndpointByMethod[TMethod],
    TEndpoint extends EndpointByMethod[TMethod][TPath],
  >(
    method: TMethod,
    path: TPath,
    ...params: MaybeOptionalArg<
      TEndpoint extends { parameters: infer UParams }
        ? NotNever<UParams> extends true
          ? UParams & { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
          : { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
        : { overrides?: RequestInit; withResponse?: true; throwOnStatusError?: boolean }
    >
  ): Promise<SafeApiResponse<TEndpoint>>;

  request<
    TMethod extends keyof EndpointByMethod,
    TPath extends keyof EndpointByMethod[TMethod],
    TEndpoint extends EndpointByMethod[TMethod][TPath],
  >(method: TMethod, path: TPath, ...params: MaybeOptionalArg<any>): Promise<any> {
    const requestParams = params[0];
    const withResponse = requestParams?.withResponse;
    const {
      withResponse: _,
      throwOnStatusError = withResponse ? false : true,
      overrides,
      ...fetchParams
    } = requestParams || {};

    const parametersToSend: EndpointParameters = {};
    if (requestParams?.body !== undefined) (parametersToSend as any).body = requestParams.body;
    if (requestParams?.query !== undefined) (parametersToSend as any).query = requestParams.query;
    if (requestParams?.header !== undefined) (parametersToSend as any).header = requestParams.header;
    if (requestParams?.path !== undefined) (parametersToSend as any).path = requestParams.path;

    const resolvedPath = (this.fetcher.decodePathParams ?? this.defaultDecodePathParams)(
      this.baseUrl + (path as string),
      (parametersToSend.path ?? {}) as Record<string, string>,
    );
    const url = new URL(resolvedPath);
    const urlSearchParams = (this.fetcher.encodeSearchParams ?? this.defaultEncodeSearchParams)(parametersToSend.query);

    const promise = this.fetcher
      .fetch({
        method: method,
        path: path as string,
        url,
        urlSearchParams,
        parameters: Object.keys(fetchParams).length ? fetchParams : undefined,
        overrides,
        throwOnStatusError,
      })
      .then(async (response) => {
        const data = await (this.fetcher.parseResponseData ?? this.defaultParseResponseData)(response);
        const typedResponse = Object.assign(response, {
          data: data,
          json: () => Promise.resolve(data),
        }) as SafeApiResponse<TEndpoint>;

        if (throwOnStatusError && errorStatusCodes.includes(response.status as never)) {
          throw new TypedStatusError(typedResponse as never);
        }

        return withResponse ? typedResponse : data;
      });

    return promise as Extract<InferResponseByStatus<TEndpoint, SuccessStatusCode>, { data: {} }>["data"];
  }
  // </ApiClient.request>
}

export function createApiClient(fetcher: Fetcher, baseUrl?: string) {
  return new ApiClient(fetcher).setBaseUrl(baseUrl ?? "");
}

/**
 Example usage:
 const api = createApiClient((method, url, params) =>
   fetch(url, { method, body: JSON.stringify(params) }).then((res) => res.json()),
 );
 api.get("/users").then((users) => console.log(users));
 api.post("/users", { body: { name: "John" } }).then((user) => console.log(user));
 api.put("/users/:id", { path: { id: 1 }, body: { name: "John" } }).then((user) => console.log(user));

 // With error handling
 const result = await api.get("/users/{id}", { path: { id: "123" }, withResponse: true });
 if (result.ok) {
   // Access data directly
   const user = result.data;
   console.log(user);

   // Or use the json() method for compatibility
   const userFromJson = await result.json();
   console.log(userFromJson);
 } else {
   const error = result.data;
   console.error(`Error ${result.status}:`, error);
 }
*/

// </ApiClient>
