use capitainerie_handlers::{errors::ApiErrorResponse, health};
use capitainerie_handlers_cards::{
    self as cards,
    list_cards::CardListResponse,
    move_card::MoveCardRequest,
    response::{CardResponse, ColumnDto, ProvenanceResponse},
};
use capitainerie_handlers_identity::{
    self as identity,
    me::{MeOrganization, MeResponse},
};
use capitainerie_handlers_runs::{
    self as runs,
    list_runs::RunListResponse,
    response::{RunResponse, RunStatusResponse},
    submit_transcript::{SubmitTranscriptRequest, SubmitTranscriptResponse},
    timeline::{
        DecisionResponse, RunTimelineResponse, StepResponse, StepStatusResponse, ToolCallResponse,
        UsageTotals,
    },
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
        runs::submit_transcript::handler,
        runs::list_runs::handler,
        runs::get_run::handler,
        runs::timeline::handler,
        cards::list_cards::handler,
        cards::move_card::handler,
    ),
    components(schemas(
        ApiErrorResponse,
        MeResponse,
        MeOrganization,
        SubmitTranscriptRequest,
        SubmitTranscriptResponse,
        RunListResponse,
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
        ProvenanceResponse,
        MoveCardRequest,
    )),
    modifiers(&BearerAuth),
    tags(
        (name = health::TAG, description = "Liveness and readiness probes"),
        (name = identity::TAG, description = "The authenticated caller"),
        (name = runs::TAG, description = "Transcripts and the runs processing them"),
        (name = cards::TAG, description = "The Kanban board and its human validation"),
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
}
