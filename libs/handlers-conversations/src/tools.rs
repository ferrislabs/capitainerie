use std::sync::Arc;

use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        authorization::Subject,
        conversation::turn::{LocalTools, ToolSpec},
        execution::ports::CodeHost,
    },
};
use serde_json::{Value, json};

pub const LIST_REPOSITORIES: &str = "list_repositories";
const MAX_REPOSITORIES: usize = 100;

/// The Captain's tools the API serves itself. Each call is decided by the
/// policy decision point as the Captain acting for the member.
pub struct ApiTools<H> {
    pub usecase: CapitainerieUseCase,
    pub code_host: Arc<H>,
    pub subject: Subject,
}

impl<H: CodeHost> LocalTools for ApiTools<H> {
    fn definitions(&self) -> Vec<ToolSpec> {
        vec![ToolSpec {
            name: LIST_REPOSITORIES.to_owned(),
            description: "List the GitHub repositories (owner/name) the organization's installation can reach. Read-only. Use it when the mission targets a repository.".to_owned(),
            parameters: json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        }]
    }

    async fn call(&self, name: &str, _: &Value) -> Result<String, String> {
        if name != LIST_REPOSITORIES {
            return Err(format!("unknown tool {name}"));
        }
        let installation = self
            .usecase
            .read_github_installation(self.subject.clone())
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| {
                "github_not_connected: this organization has not connected GitHub".to_owned()
            })?;
        let repositories = self
            .code_host
            .repositories(self.subject.organization_id, installation.installation_id)
            .await
            .map_err(|error| error.to_string())?;
        if repositories.is_empty() {
            return Ok("The installation reaches no repository.".to_owned());
        }
        Ok(repositories
            .iter()
            .take(MAX_REPOSITORIES)
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"))
    }
}
