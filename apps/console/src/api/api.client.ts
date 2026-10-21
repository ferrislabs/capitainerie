export namespace Schemas {
  // <Schemas>
  export type ActiveStepResponse = {
    agent?: (string | null) | undefined;
    kind: string;
    model?: (string | null) | undefined;
    started_at: string;
  };
  export type CardId = string;
  export type ClientId = string;
  export type ClientRefResponse = { id: ClientId; name: string };
  export type RunId = string;
  export type ProjectId = string;
  export type ProjectRefResponse = { id: ProjectId; name: string };
  export type RunStatusResponse = "queued" | "running" | "succeeded" | "failed" | "cancelled";
  export type TranscriptId = string;
  export type RunResponse = {
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
  export type AgentResponse = {
    agent: string;
    cost_micros: number;
    input_tokens: number;
    last_seen: string;
    output_tokens: number;
    runs: number;
  };
  export type ApiErrorResponse = { code: string; message: string; status: number };
  export type CapabilitiesResponse = { chat: boolean; embeddings: boolean; ocr: boolean; transcription: boolean };
  export type CardCommentId = string;
  export type CriterionDto = { done: boolean; text: string };
  export type ColumnDto = "proposed" | "todo" | "in_progress" | "done" | "rejected";
  export type EstimateDto = "xs" | "s" | "m" | "l" | "xl";
  export type PriorityDto = "low" | "normal" | "high" | "urgent";
  export type ProvenanceResponse = { end: number; quote: string; start: number; transcript_id: TranscriptId };
  export type CardResponse = {
    acceptance_criteria: Array<CriterionDto>;
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
  export type CreateProjectRequest = {
    aliases?: (Array<string> | null) | undefined;
    client_id?: (null | ClientId) | undefined;
    description?: (string | null) | undefined;
    name: string;
    repository?: (string | null) | undefined;
  };
  export type CreateTranscriptionRequest = { file: string };
  export type CreatedConversationResponse = { id: ConversationId };
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
  export type InstallUrlResponse = { url: string };
  export type IntegrationInstanceListResponse = { items: Array<IntegrationInstanceResponse> };
  export type ManifestResponse = { action_url: string; manifest: string; state: string };
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
  export type MoveCardRequest = { expected_version: number; to: ColumnDto };
  export type PatchCardRequest = {
    acceptance_criteria?: (Array<CriterionDto> | null) | undefined;
    assignee_subject?: (string | null) | undefined;
    client_id?: (null | ClientId) | undefined;
    description?: (string | null) | undefined;
    due_date?: (string | null) | undefined;
    estimate?: (null | EstimateDto) | undefined;
    expected_version: number;
    labels?: (Array<string> | null) | undefined;
    priority?: (null | PriorityDto) | undefined;
    project_id?: (null | ProjectId) | undefined;
    title?: (string | null) | undefined;
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
  export type ReadDocumentRequest = { file: string };
  export type RepositoryResponse = { full_name: string; name: string; owner: string };
  export type RepositoryListResponse = { repositories: Array<RepositoryResponse> };
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
  export type SendMessageRequest = Partial<{ message: string | null }>;
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
  export type TestProviderResponse = { error?: (string | null) | undefined; models: Array<ModelResponse>; ok: boolean };
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
  export type UpdateProjectRequest = Partial<{
    aliases: Array<string> | null;
    archived: boolean | null;
    client_id: null | ClientId;
    description: string | null;
    name: string | null;
    repository: string | null;
  }>;

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
  export type get_ListCards = {
    method: "GET";
    path: "/api/v1/cards";
    requestFormat: "json";
    parameters: {
      query: Partial<{ client_id: string; project_id: string; priority: string; assignee: string; label: string }>;
    };
    responses: { 200: Schemas.CardListResponse; 401: Schemas.ApiErrorResponse; 422: Schemas.ApiErrorResponse };
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
  export type get_ListGithubRepositories = {
    method: "GET";
    path: "/api/v1/integrations/github/repositories";
    requestFormat: "json";
    parameters: never;
    responses: { 200: Schemas.RepositoryListResponse; 401: Schemas.ApiErrorResponse; 409: Schemas.ApiErrorResponse };
  };
  export type get_GetMe = {
    method: "GET";
    path: "/api/v1/me";
    requestFormat: "json";
    parameters: never;
    responses: { 200: Schemas.MeResponse; 401: Schemas.ApiErrorResponse };
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
  export type get_ListRuns = {
    method: "GET";
    path: "/api/v1/runs";
    requestFormat: "json";
    parameters: {
      query: Partial<{ limit: number; offset: number }>;
    };
    responses: { 200: Schemas.RunListResponse; 401: Schemas.ApiErrorResponse };
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
  export type get_GetRunTimeline = {
    method: "GET";
    path: "/api/v1/runs/{run_id}/timeline";
    requestFormat: "json";
    parameters: {
      path: { run_id: string };
    };
    responses: { 200: Schemas.RunTimelineResponse; 401: Schemas.ApiErrorResponse; 404: Schemas.ApiErrorResponse };
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

  // </Endpoints>
}

// <EndpointByMethod>
export type EndpointByMethod = {
  get: {
    "/api/v1/agents": Endpoints.get_ListAgents;
    "/api/v1/analytics/costs": Endpoints.get_GetCosts;
    "/api/v1/cards": Endpoints.get_ListCards;
    "/api/v1/cards/{card_id}": Endpoints.get_GetCard;
    "/api/v1/clients": Endpoints.get_ListClients;
    "/api/v1/clients/{client_id}": Endpoints.get_GetClient;
    "/api/v1/clients/{client_id}/integrations": Endpoints.get_ListIntegrationInstances;
    "/api/v1/conversations": Endpoints.get_ListConversations;
    "/api/v1/conversations/{id}": Endpoints.get_GetConversation;
    "/api/v1/integrations/github": Endpoints.get_GetGithubIntegration;
    "/api/v1/integrations/github/install": Endpoints.get_GetGithubInstallUrl;
    "/api/v1/integrations/github/repositories": Endpoints.get_ListGithubRepositories;
    "/api/v1/me": Endpoints.get_GetMe;
    "/api/v1/members": Endpoints.get_ListMembers;
    "/api/v1/model-assignments": Endpoints.get_GetModelAssignments;
    "/api/v1/models": Endpoints.get_ListModels;
    "/api/v1/projects": Endpoints.get_ListProjects;
    "/api/v1/projects/{project_id}": Endpoints.get_GetProject;
    "/api/v1/providers": Endpoints.get_ListProviders;
    "/api/v1/runs": Endpoints.get_ListRuns;
    "/api/v1/runs/active": Endpoints.get_ListActiveRuns;
    "/api/v1/runs/{run_id}": Endpoints.get_GetRun;
    "/api/v1/runs/{run_id}/timeline": Endpoints.get_GetRunTimeline;
    "/health/live": Endpoints.get_GetLiveness;
    "/health/ready": Endpoints.get_GetReadiness;
  };
  patch: {
    "/api/v1/cards/{card_id}": Endpoints.patch_PatchCard;
    "/api/v1/clients/{client_id}": Endpoints.patch_UpdateClient;
    "/api/v1/conversations/{id}": Endpoints.patch_UpdateConversation;
    "/api/v1/integration-instances/{id}": Endpoints.patch_UpdateIntegrationInstance;
    "/api/v1/projects/{project_id}": Endpoints.patch_UpdateProject;
  };
  post: {
    "/api/v1/cards/{card_id}/comments": Endpoints.post_CommentCard;
    "/api/v1/cards/{card_id}/executions": Endpoints.post_ExecuteCard;
    "/api/v1/cards/{card_id}/move": Endpoints.post_MoveCard;
    "/api/v1/clients": Endpoints.post_CreateClient;
    "/api/v1/clients/{client_id}/integrations": Endpoints.post_CreateIntegrationInstance;
    "/api/v1/conversations": Endpoints.post_CreateConversation;
    "/api/v1/conversations/{id}/messages": Endpoints.post_SendConversationMessage;
    "/api/v1/documents/text": Endpoints.post_ReadDocument;
    "/api/v1/integration-instances/{id}/check": Endpoints.post_CheckIntegrationInstance;
    "/api/v1/integrations/github/app": Endpoints.post_CreateGithubApp;
    "/api/v1/integrations/github/app/manifest": Endpoints.post_CreateGithubAppManifest;
    "/api/v1/integrations/github/callback": Endpoints.post_ConnectGithub;
    "/api/v1/projects": Endpoints.post_CreateProject;
    "/api/v1/providers/{kind}/test": Endpoints.post_TestProvider;
    "/api/v1/transcriptions": Endpoints.post_CreateTranscription;
    "/api/v1/transcripts": Endpoints.post_SubmitTranscript;
  };
  delete: {
    "/api/v1/integration-instances/{id}": Endpoints.delete_ArchiveIntegrationInstance;
    "/api/v1/integrations/github": Endpoints.delete_DisconnectGithub;
    "/api/v1/integrations/github/app": Endpoints.delete_DeleteGithubApp;
    "/api/v1/providers/{kind}": Endpoints.delete_DeleteProvider;
  };
  put: {
    "/api/v1/model-assignments": Endpoints.put_PutModelAssignments;
    "/api/v1/providers/{kind}": Endpoints.put_PutProvider;
  };
};

// </EndpointByMethod>

// <EndpointByMethod.Shorthands>
export type GetEndpoints = EndpointByMethod["get"];
export type PatchEndpoints = EndpointByMethod["patch"];
export type PostEndpoints = EndpointByMethod["post"];
export type DeleteEndpoints = EndpointByMethod["delete"];
export type PutEndpoints = EndpointByMethod["put"];
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
