use axum::{
    Extension, Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::routing::TypedPath;
use capitainerie_common::{CardId, CoreError, IntegrationInstanceId, RunId};
use capitainerie_core::domain::integration::{
    action::Access,
    intervention::{ConfideCard, ConfideRefusal, InterventionError},
};
use capitainerie_handlers::{ApiError, AppState, Caller, errors::ApiErrorResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::response::CardResponse;

pub const MAX_GRANTS: usize = 100;

#[derive(TypedPath, Deserialize)]
#[typed_path("/api/v1/cards/{card_id}/interventions")]
pub struct InterveneCardPath {
    pub card_id: CardId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GrantAccessDto {
    Read,
    Write,
}

impl From<GrantAccessDto> for Access {
    fn from(access: GrantAccessDto) -> Self {
        match access {
            GrantAccessDto::Read => Self::Read,
            GrantAccessDto::Write => Self::Write,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct InstanceGrantRequest {
    pub instance_id: IntegrationInstanceId,
    pub access: GrantAccessDto,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct InterveneCardRequest {
    pub instructions: Option<String>,
    pub grants: Vec<InstanceGrantRequest>,
    #[serde(default)]
    pub confirm_production_writes: bool,
    pub expected_version: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InterveneCardResponse {
    pub run_id: RunId,
    pub card: CardResponse,
}

#[derive(Debug)]
pub enum InterveneError {
    Api(ApiError),
    Refused(ConfideRefusal),
}

impl From<ApiError> for InterveneError {
    fn from(error: ApiError) -> Self {
        Self::Api(error)
    }
}

impl From<CoreError> for InterveneError {
    fn from(error: CoreError) -> Self {
        Self::Api(error.into())
    }
}

impl From<InterventionError> for InterveneError {
    fn from(error: InterventionError) -> Self {
        match error {
            InterventionError::Core(error) => error.into(),
            InterventionError::Refused(refusal) => Self::Refused(refusal),
        }
    }
}

impl IntoResponse for InterveneError {
    fn into_response(self) -> Response {
        match self {
            Self::Api(error) => error.into_response(),
            Self::Refused(refusal) => {
                let status = StatusCode::UNPROCESSABLE_ENTITY;
                let body = ApiErrorResponse {
                    code: refusal.code(),
                    status: status.as_u16(),
                    message: refusal.to_string(),
                };
                (status, Json(body)).into_response()
            }
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/cards/{card_id}/interventions",
    operation_id = "interveneCard",
    tag = super::TAG,
    params(("card_id" = CardId, Path, description = "Card identifier")),
    request_body = InterveneCardRequest,
    responses(
        (status = 202, description = "The card is In progress and an intervention run is queued", body = InterveneCardResponse),
        (status = 401, description = "Missing or invalid token", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 403, description = "The policy refused: an agent cannot hand a card over", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 404, description = "No such card in the caller's organization", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 409, description = "card_not_executable, execution_already_active, or the card changed since it was read", body = capitainerie_handlers::errors::ApiErrorResponse),
        (status = 422, description = "card_without_client, no_instance_granted, instance_of_another_client, instance_archived, production_write_not_confirmed, instructions_too_long, or more than 100 grants", body = capitainerie_handlers::errors::ApiErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn handler(
    InterveneCardPath { card_id }: InterveneCardPath,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Json(request): Json<InterveneCardRequest>,
) -> Result<(StatusCode, Json<InterveneCardResponse>), InterveneError> {
    if request.grants.len() > MAX_GRANTS {
        return Err(ApiError::BadRequest(format!("at most {MAX_GRANTS} grants")).into());
    }
    let command = ConfideCard {
        instructions: request.instructions.unwrap_or_default(),
        grants: request
            .grants
            .into_iter()
            .map(|grant| (grant.instance_id, grant.access.into()))
            .collect(),
        confirm_production_writes: request.confirm_production_writes,
        expected_version: request.expected_version,
    };
    let started = state
        .usecase
        .confide_card_to_integrator(caller.subject(), card_id, command)
        .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(InterveneCardResponse {
            run_id: started.run_id,
            card: started.card.into(),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;

    use super::*;

    async fn rendered(error: impl Into<InterveneError>) -> (StatusCode, serde_json::Value) {
        let response = error.into().into_response();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    #[tokio::test]
    async fn a_refusal_is_a_422_with_its_code() {
        let cases = [
            (ConfideRefusal::NoClient, "card_without_client"),
            (ConfideRefusal::NoGrant, "no_instance_granted"),
            (
                ConfideRefusal::InstanceOfAnotherClient,
                "instance_of_another_client",
            ),
            (ConfideRefusal::InstanceArchived, "instance_archived"),
            (
                ConfideRefusal::ProductionWriteNotConfirmed,
                "production_write_not_confirmed",
            ),
            (ConfideRefusal::InstructionsTooLong, "instructions_too_long"),
        ];
        for (refusal, code) in cases {
            let message = refusal.to_string();
            let (status, body) = rendered(InterventionError::Refused(refusal)).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(body["code"], code);
            assert_eq!(body["status"], 422);
            assert_eq!(body["message"], message);
        }
    }

    #[tokio::test]
    async fn a_core_error_keeps_its_usual_mapping() {
        let cases = [
            (CoreError::NotFound, StatusCode::NOT_FOUND, "not_found"),
            (
                CoreError::Conflict("execution_already_active".into()),
                StatusCode::CONFLICT,
                "conflict",
            ),
            (
                CoreError::Forbidden { reason: None },
                StatusCode::FORBIDDEN,
                "forbidden",
            ),
        ];
        for (error, expected, code) in cases {
            let (status, body) = rendered(InterventionError::Core(error)).await;
            assert_eq!(status, expected);
            assert_eq!(body["code"], code);
        }
    }

    #[test]
    fn the_request_defaults_to_no_instructions_and_no_production_confirmation() {
        let request: InterveneCardRequest = serde_json::from_value(serde_json::json!({
            "grants": [{"instance_id": "0198f2a0-0000-7000-8000-000000000001", "access": "write"}],
            "expected_version": 3,
        }))
        .unwrap();

        assert_eq!(request.instructions, None);
        assert!(!request.confirm_production_writes);
        assert_eq!(request.grants[0].access, GrantAccessDto::Write);
        assert_eq!(Access::from(request.grants[0].access), Access::Write);
    }
}
