pub mod dto;
pub mod list_for_card;
pub mod list_for_run;

use capitainerie_common::{CoreError, IntegrationInstanceId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{authorization::Subject, integration::journal::IntegrationAction},
};

use self::dto::{IntegrationActionListResponse, IntegrationActionResponse};

pub(crate) async fn describe(
    usecase: &CapitainerieUseCase,
    subject: Subject,
    actions: Vec<IntegrationAction>,
) -> Result<IntegrationActionListResponse, CoreError> {
    let mut ids: Vec<IntegrationInstanceId> = Vec::new();
    for action in &actions {
        if !ids.contains(&action.instance_id) {
            ids.push(action.instance_id);
        }
    }
    let instances = usecase.integration_instances_by_id(subject, &ids).await?;
    let items = actions
        .into_iter()
        .map(|action| {
            let instance = instances
                .iter()
                .find(|instance| instance.id == action.instance_id)
                .ok_or_else(|| {
                    CoreError::Internal(format!(
                        "the journal row {} points to an unknown instance",
                        action.id
                    ))
                })?;
            Ok(IntegrationActionResponse::new(action, instance))
        })
        .collect::<Result<_, CoreError>>()?;
    Ok(IntegrationActionListResponse { items })
}
