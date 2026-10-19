use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_common::OrganizationId;
use capitainerie_core::domain::member::Member;
use capitainerie_handlers::{ApiError, AppState, Caller};
use chrono::Utc;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(TypedPath)]
#[typed_path("/api/v1/me")]
pub struct MePath;

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
        (status = 200, description = "The caller and their organization; a human caller is also recorded as a member of the organization, whom cards can be assigned to", body = MeResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: MePath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<MeResponse>, ApiError> {
    if caller.identity.actor.is_none() {
        state.usecase.record_member(member_of(&caller)).await?;
    }
    Ok(Json(caller.into()))
}

fn member_of(caller: &Caller) -> Member {
    let identity = &caller.identity;
    let name = [
        identity.display_name.as_deref(),
        identity.username.as_deref(),
        identity.email.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .find(|name| !name.is_empty())
    .unwrap_or(&identity.subject);
    Member {
        organization_id: caller.organization.id,
        subject: identity.subject.clone(),
        name: name.to_owned(),
        email: identity.email.clone(),
        last_seen_at: Utc::now(),
    }
}
