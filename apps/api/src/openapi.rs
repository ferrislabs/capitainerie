use capitainerie_handlers::{errors::ApiErrorResponse, health};
use capitainerie_handlers_identity::{
    self as identity,
    me::{MeOrganization, MeResponse},
};
use utoipa::{
    Modify, OpenApi,
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
};

#[derive(OpenApi)]
#[openapi(
    info(title = "Capitainerie API"),
    paths(health::live, health::ready, identity::me::handler),
    components(schemas(ApiErrorResponse, MeResponse, MeOrganization)),
    modifiers(&BearerAuth),
    tags(
        (name = health::TAG, description = "Liveness and readiness probes"),
        (name = identity::TAG, description = "The authenticated caller"),
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
}
