use axum::{Extension, Json, extract::State};
use axum_extra::routing::TypedPath;
use capitainerie_core::domain::member::Member;
use capitainerie_handlers::{ApiError, AppState, Caller};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(TypedPath)]
#[typed_path("/api/v1/members")]
pub struct MembersPath;

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberResponse {
    /// FerrisKey subject: what `assignee_subject` takes.
    pub subject: String,
    pub name: String,
    pub email: Option<String>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberListResponse {
    pub items: Vec<MemberResponse>,
}

impl From<Member> for MemberResponse {
    fn from(member: Member) -> Self {
        Self {
            subject: member.subject,
            name: member.name,
            email: member.email,
            last_seen_at: member.last_seen_at,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/members",
    operation_id = "listMembers",
    tag = super::TAG,
    responses(
        (status = 200, description = "The humans of the organization who have signed in to the console, by name", body = MemberListResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused the read; agents have no member access", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    _: MembersPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<MemberListResponse>, ApiError> {
    let members = state.usecase.list_members(caller.subject()).await?;
    Ok(Json(MemberListResponse {
        items: members.into_iter().map(Into::into).collect(),
    }))
}
