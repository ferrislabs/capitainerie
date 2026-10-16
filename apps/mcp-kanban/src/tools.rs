use axum::http::request::Parts;
use capitainerie_common::{CardId, CoreError, RunId, TranscriptId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        authorization::Subject,
        card::{
            Card, CardComment, Column, Provenance,
            service::{CreateCard, MoveCard, ProvenanceInput, UpdateCard},
        },
    },
};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Extension, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerConfig},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::into_tool_result;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ProvenanceParam {
    #[schemars(description = "UUID of the transcript the card comes from.")]
    pub transcript_id: String,
    #[schemars(
        description = "Start of the quoted passage: a character offset (not a byte offset) into the transcript body, inclusive."
    )]
    pub start: u32,
    #[schemars(
        description = "End of the quoted passage: a character offset into the transcript body, exclusive."
    )]
    pub end: u32,
    #[schemars(
        description = "The exact text of the transcript between start and end. It is checked against the transcript: a quote that does not match is refused."
    )]
    pub quote: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct CreateCardInput {
    #[schemars(description = "Short title of the card, at most 200 characters.")]
    pub title: String,
    #[schemars(description = "What has to be done and why, at most 10000 characters.")]
    pub description: String,
    #[schemars(description = "UUID of the run you are working for, when you were given one.")]
    pub run_id: Option<String>,
    #[schemars(
        description = "Where in the transcript this card comes from. Strongly recommended: a card without provenance cannot be traced back to what was said."
    )]
    pub provenance: Option<ProvenanceParam>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct UpdateCardInput {
    #[schemars(description = "UUID of the card to change.")]
    pub card_id: String,
    #[schemars(
        description = "The version of the card from your last read or from the result of your last change to it. The change is refused if the card has moved on since."
    )]
    pub expected_version: i32,
    #[schemars(description = "New title. Omit to keep the current one.")]
    pub title: Option<String>,
    #[schemars(description = "New description. Omit to keep the current one.")]
    pub description: Option<String>,
    #[schemars(description = "UUID of the run you are working for, when you were given one.")]
    pub run_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct CommentInput {
    #[schemars(description = "UUID of the card to comment on.")]
    pub card_id: String,
    #[schemars(description = "The comment, at most 5000 characters.")]
    pub body: String,
    #[schemars(description = "UUID of the run you are working for, when you were given one.")]
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(rename_all = "snake_case")]
pub enum ColumnParam {
    Proposed,
    Todo,
    InProgress,
    Done,
    Rejected,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct MoveCardInput {
    #[schemars(description = "UUID of the card to move.")]
    pub card_id: String,
    #[schemars(description = "The column to move the card to.")]
    pub to: ColumnParam,
    #[schemars(
        description = "The version of the card from your last read or from the result of your last change to it. The move is refused if the card has moved on since."
    )]
    pub expected_version: i32,
    #[schemars(description = "UUID of the run you are working for, when you were given one.")]
    pub run_id: Option<String>,
}

impl From<ColumnParam> for Column {
    fn from(param: ColumnParam) -> Self {
        match param {
            ColumnParam::Proposed => Self::Proposed,
            ColumnParam::Todo => Self::Todo,
            ColumnParam::InProgress => Self::InProgress,
            ColumnParam::Done => Self::Done,
            ColumnParam::Rejected => Self::Rejected,
        }
    }
}

fn uuid(value: &str, what: &str) -> Result<Uuid, CoreError> {
    Uuid::parse_str(value.trim())
        .map_err(|_| CoreError::Invalid(format!("{what} is not a valid UUID: {value}")))
}

fn card_id(value: &str) -> Result<CardId, CoreError> {
    uuid(value, "card_id").map(CardId)
}

fn run_id(value: Option<&str>) -> Result<Option<RunId>, CoreError> {
    value
        .map(|value| uuid(value, "run_id").map(RunId))
        .transpose()
}

impl TryFrom<ProvenanceParam> for ProvenanceInput {
    type Error = CoreError;

    fn try_from(param: ProvenanceParam) -> Result<Self, Self::Error> {
        Ok(Self {
            transcript_id: TranscriptId(uuid(&param.transcript_id, "transcript_id")?),
            start: param.start,
            end: param.end,
            quote: param.quote,
        })
    }
}

impl TryFrom<CreateCardInput> for CreateCard {
    type Error = CoreError;

    fn try_from(input: CreateCardInput) -> Result<Self, Self::Error> {
        Ok(Self {
            run_id: run_id(input.run_id.as_deref())?,
            provenance: input
                .provenance
                .map(ProvenanceInput::try_from)
                .transpose()?,
            title: input.title,
            description: input.description,
        })
    }
}

impl TryFrom<&UpdateCardInput> for UpdateCard {
    type Error = CoreError;

    fn try_from(input: &UpdateCardInput) -> Result<Self, Self::Error> {
        Ok(Self {
            title: input.title.clone(),
            description: input.description.clone(),
            expected_version: input.expected_version,
            run_id: run_id(input.run_id.as_deref())?,
        })
    }
}

impl TryFrom<&MoveCardInput> for MoveCard {
    type Error = CoreError;

    fn try_from(input: &MoveCardInput) -> Result<Self, Self::Error> {
        Ok(Self {
            to: input.to.into(),
            expected_version: input.expected_version,
            run_id: run_id(input.run_id.as_deref())?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct ProvenanceView {
    pub transcript_id: Uuid,
    pub start: u32,
    pub end: u32,
    pub quote: String,
}

#[derive(Debug, Serialize)]
pub struct CardView {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub column: &'static str,
    pub version: i32,
    pub provenance: Option<ProvenanceView>,
    pub created_by_run: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct CommentView {
    pub id: Uuid,
    pub card_id: Uuid,
    pub body: String,
    pub created_at: String,
}

impl From<&Provenance> for ProvenanceView {
    fn from(provenance: &Provenance) -> Self {
        Self {
            transcript_id: provenance.transcript_id.0,
            start: provenance.start,
            end: provenance.end,
            quote: provenance.quote.clone(),
        }
    }
}

impl From<&Card> for CardView {
    fn from(card: &Card) -> Self {
        Self {
            id: card.id.0,
            title: card.title.as_str().to_owned(),
            description: card.description.clone(),
            column: card.column.as_str(),
            version: card.version,
            provenance: card.provenance.as_ref().map(ProvenanceView::from),
            created_by_run: card.created_by_run.map(|run| run.0),
        }
    }
}

impl From<&CardComment> for CommentView {
    fn from(comment: &CardComment) -> Self {
        Self {
            id: comment.id.0,
            card_id: comment.card_id.0,
            body: comment.body.clone(),
            created_at: comment.created_at.to_rfc3339(),
        }
    }
}

#[derive(Clone)]
pub struct KanbanTools {
    usecase: CapitainerieUseCase,
    tool_router: ToolRouter<Self>,
}

impl KanbanTools {
    pub fn new(usecase: CapitainerieUseCase) -> Self {
        Self {
            usecase,
            tool_router: Self::tool_router(),
        }
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.tool_router
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect()
    }
}

fn subject_of(parts: &Parts) -> Result<Subject, ErrorData> {
    let subject = parts.extensions.get::<Subject>().cloned().ok_or_else(|| {
        ErrorData::internal_error("no authenticated subject on the request", None)
    })?;
    let span = tracing::Span::current();
    span.record("organization_id", subject.organization_id.0.to_string());
    span.record("actor", subject.actor.as_deref().unwrap_or("-"));
    Ok(subject)
}

#[tool_router(router = tool_router)]
impl KanbanTools {
    #[tool(
        description = "Propose a new card on the Kanban board. Whatever you ask, an agent-created card always lands in the \"proposed\" column: only a human can accept it into \"todo\" or reject it. Give a short actionable title and a description saying what to do and why. Pass provenance (transcript_id, character offsets start/end, and the exact quote) whenever the card comes from a transcript: the quote is verified against the transcript and a mismatch is refused. Returns the card with its id and its version."
    )]
    #[tracing::instrument(
        skip_all,
        fields(gen_ai.tool.name = "create_card", organization_id, actor)
    )]
    async fn create_card(
        &self,
        Parameters(input): Parameters<CreateCardInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let subject = subject_of(&parts)?;
        let command = match CreateCard::try_from(input) {
            Ok(command) => command,
            Err(error) => return into_tool_result::<()>(Err(error)),
        };
        into_tool_result(
            self.usecase
                .create_card(subject, command)
                .await
                .map(|card| CardView::from(&card)),
        )
    }

    #[tool(
        description = "Change the title and/or the description of an existing card. Omitted fields are kept. expected_version is the optimistic-lock version of the card from your last read or from the result of your last change: if someone changed the card since, the call fails with a conflict and you must re-read the card before retrying. This does not move the card between columns. Returns the updated card with its new version."
    )]
    #[tracing::instrument(
        skip_all,
        fields(gen_ai.tool.name = "update_card", organization_id, actor)
    )]
    async fn update_card(
        &self,
        Parameters(input): Parameters<UpdateCardInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let subject = subject_of(&parts)?;
        let prepared =
            card_id(&input.card_id).and_then(|id| Ok((id, UpdateCard::try_from(&input)?)));
        let (id, command) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => return into_tool_result::<()>(Err(error)),
        };
        into_tool_result(
            self.usecase
                .update_card(subject, id, command)
                .await
                .map(|card| CardView::from(&card)),
        )
    }

    #[tool(
        description = "Add a comment to a card, for example to explain why you proposed it or to flag a doubt for the human who will triage it. A comment never changes the card or its version. Returns the comment."
    )]
    #[tracing::instrument(skip_all, fields(gen_ai.tool.name = "comment", organization_id, actor))]
    async fn comment(
        &self,
        Parameters(input): Parameters<CommentInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let subject = subject_of(&parts)?;
        let prepared =
            card_id(&input.card_id).and_then(|id| Ok((id, run_id(input.run_id.as_deref())?)));
        let (id, run) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => return into_tool_result::<()>(Err(error)),
        };
        into_tool_result(
            self.usecase
                .comment_card(subject, id, input.body, run)
                .await
                .map(|comment| CommentView::from(&comment)),
        )
    }

    #[tool(
        description = "Move a card to another column: proposed, todo, in_progress, done or rejected. Only a human can take a card out of \"proposed\": an agent trying to do so gets a forbidden error, and the card stays where it is. expected_version is the optimistic-lock version of the card from your last read or from the result of your last change to it; a stale version fails with a conflict. Returns the card with its new version."
    )]
    #[tracing::instrument(skip_all, fields(gen_ai.tool.name = "move_card", organization_id, actor))]
    async fn move_card(
        &self,
        Parameters(input): Parameters<MoveCardInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let subject = subject_of(&parts)?;
        let prepared = card_id(&input.card_id).and_then(|id| Ok((id, MoveCard::try_from(&input)?)));
        let (id, command) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => return into_tool_result::<()>(Err(error)),
        };
        into_tool_result(
            self.usecase
                .move_card(subject, id, command)
                .await
                .map(|card| CardView::from(&card)),
        )
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for KanbanTools {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Capitainerie Kanban. Propose cards from meeting transcripts with create_card (they land in \"proposed\" until a human triages them), refine them with update_card and comment, and move them with move_card within the limits of your role.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0190f0c1-0000-7000-8000-000000000001";

    fn create(provenance: Option<ProvenanceParam>, run: Option<&str>) -> CreateCardInput {
        CreateCardInput {
            title: "Relancer Acme".into(),
            description: "Avant vendredi".into(),
            run_id: run.map(str::to_owned),
            provenance,
        }
    }

    #[test]
    fn create_input_converts_with_provenance_and_run() {
        let command = CreateCard::try_from(create(
            Some(ProvenanceParam {
                transcript_id: ID.into(),
                start: 3,
                end: 9,
                quote: "Acme".into(),
            }),
            Some(ID),
        ))
        .unwrap();
        let provenance = command.provenance.unwrap();
        assert_eq!(provenance.transcript_id.0.to_string(), ID);
        assert_eq!((provenance.start, provenance.end), (3, 9));
        assert_eq!(command.run_id.unwrap().0.to_string(), ID);
        assert_eq!(command.title, "Relancer Acme");
    }

    #[test]
    fn create_input_rejects_malformed_ids() {
        assert!(matches!(
            CreateCard::try_from(create(None, Some("nope"))),
            Err(CoreError::Invalid(message)) if message.contains("run_id")
        ));
        let bad = ProvenanceParam {
            transcript_id: "x".into(),
            start: 0,
            end: 1,
            quote: "a".into(),
        };
        assert!(CreateCard::try_from(create(Some(bad), None)).is_err());
    }

    #[test]
    fn move_input_converts_the_column_and_version() {
        let input = MoveCardInput {
            card_id: ID.into(),
            to: ColumnParam::InProgress,
            expected_version: 4,
            run_id: None,
        };
        let command = MoveCard::try_from(&input).unwrap();
        assert_eq!(command.to, Column::InProgress);
        assert_eq!(command.expected_version, 4);
        assert!(card_id("zzz").is_err());
        assert_eq!(card_id(ID).unwrap().0.to_string(), ID);
    }

    #[test]
    fn update_input_keeps_omitted_fields_unset() {
        let input = UpdateCardInput {
            card_id: ID.into(),
            expected_version: 2,
            title: None,
            description: Some("d".into()),
            run_id: None,
        };
        let command = UpdateCard::try_from(&input).unwrap();
        assert_eq!(command.title, None);
        assert_eq!(command.description.as_deref(), Some("d"));
    }

    #[test]
    fn column_names_match_the_domain() {
        for column in Column::ALL {
            let param: ColumnParam =
                serde_json::from_value(serde_json::json!(column.as_str())).unwrap();
            assert_eq!(Column::from(param), column);
        }
        assert!(serde_json::from_value::<ColumnParam>(serde_json::json!("backlog")).is_err());
    }
}
