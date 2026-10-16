use axum::{Extension, Json};
use capitainerie_common::OrganizationId;
use capitainerie_handlers::{ApiError, Caller};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponse {
    pub subject: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub organization: MeOrganization,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MeOrganization {
    pub id: OrganizationId,
    pub slug: String,
    pub name: String,
}

impl From<Caller> for MeResponse {
    fn from(
        Caller {
            identity,
            organization,
            ..
        }: Caller,
    ) -> Self {
        Self {
            subject: identity.subject,
            username: identity.username,
            email: identity.email,
            display_name: identity.display_name,
            organization: MeOrganization {
                id: organization.id,
                slug: organization.slug.as_str().to_owned(),
                name: organization.name,
            },
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/me",
    operation_id = "getMe",
    tag = super::TAG,
    responses(
        (status = 200, description = "The caller and their organization", body = MeResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(Extension(caller): Extension<Caller>) -> Result<Json<MeResponse>, ApiError> {
    Ok(Json(caller.into()))
}
