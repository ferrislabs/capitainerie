use capitainerie_handlers::{errors::ApiErrorResponse, health};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(title = "Capitainerie API"),
    paths(health::live, health::ready),
    components(schemas(ApiErrorResponse)),
    tags((name = health::TAG, description = "Liveness and readiness probes"))
)]
pub struct ApiDoc;

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
}
