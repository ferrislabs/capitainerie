use capitainerie_common::{ClientId, ProjectId};
use capitainerie_core::domain::{
    client::{Client, ClientDetail, ClientListing, ColumnCount},
    project::{Project, ProjectDetail, ProjectListing},
};
use capitainerie_handlers_cards::response::ColumnDto;
use capitainerie_handlers_runs::response::ClientRefResponse;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ClientResponse {
    pub id: ClientId,
    pub name: String,
    pub aliases: Vec<String>,
    pub notes: String,
    pub archived: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClientListItemResponse {
    pub client: ClientResponse,
    pub project_count: u64,
    pub open_cards: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClientListResponse {
    pub items: Vec<ClientListItemResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ColumnCountResponse {
    pub column: ColumnDto,
    pub cards: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClientDetailResponse {
    pub client: ClientResponse,
    pub projects: Vec<ProjectResponse>,
    pub cards: Vec<ColumnCountResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectResponse {
    pub id: ProjectId,
    pub client: Option<ClientRefResponse>,
    pub name: String,
    pub aliases: Vec<String>,
    pub repository: Option<String>,
    pub description: String,
    pub archived: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectListItemResponse {
    pub project: ProjectResponse,
    pub open_cards: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectListResponse {
    pub items: Vec<ProjectListItemResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectDetailResponse {
    pub project: ProjectResponse,
    pub cards: Vec<ColumnCountResponse>,
}

impl From<Client> for ClientResponse {
    fn from(client: Client) -> Self {
        Self {
            id: client.id,
            name: client.name.as_str().to_owned(),
            aliases: client.aliases.as_slice().to_vec(),
            notes: client.notes,
            archived: client.archived_at.is_some(),
            created_by: client.created_by,
            created_at: client.created_at,
            updated_at: client.updated_at,
        }
    }
}

impl From<ClientListing> for ClientListItemResponse {
    fn from(listing: ClientListing) -> Self {
        Self {
            client: listing.client.into(),
            project_count: listing.projects,
            open_cards: listing.open_cards,
        }
    }
}

impl From<ColumnCount> for ColumnCountResponse {
    fn from(count: ColumnCount) -> Self {
        Self {
            column: count.column.into(),
            cards: count.cards,
        }
    }
}

impl From<ClientDetail> for ClientDetailResponse {
    fn from(detail: ClientDetail) -> Self {
        Self {
            client: detail.client.into(),
            projects: detail.projects.into_iter().map(Into::into).collect(),
            cards: detail.cards.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<Project> for ProjectResponse {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            client: project.client.map(|client| ClientRefResponse {
                id: client.id,
                name: client.name,
            }),
            name: project.name.as_str().to_owned(),
            aliases: project.aliases.as_slice().to_vec(),
            repository: project.repository.map(|repository| repository.to_string()),
            description: project.description,
            archived: project.archived_at.is_some(),
            created_by: project.created_by,
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}

impl From<ProjectListing> for ProjectListItemResponse {
    fn from(listing: ProjectListing) -> Self {
        Self {
            project: listing.project.into(),
            open_cards: listing.open_cards,
        }
    }
}

impl From<ProjectDetail> for ProjectDetailResponse {
    fn from(detail: ProjectDetail) -> Self {
        Self {
            project: detail.project.into(),
            cards: detail.cards.into_iter().map(Into::into).collect(),
        }
    }
}
