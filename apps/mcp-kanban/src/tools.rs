use axum::http::request::Parts;
use capitainerie_common::{CardId, CoreError, RunId, TranscriptId};
use capitainerie_core::{
    CapitainerieUseCase,
    domain::{
        authorization::Subject,
        card::{
            Card, CardComment, Column, Provenance,
            attributes::{
                AcceptanceCriteria, Attributes, Criterion, Estimate, Priority, agent_due_date,
                parse_due_date,
            },
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
    #[schemars(
        description = "Verifiable conditions that must be true when the card is done, one short sentence each (at most 20, 300 characters each). Give them when the transcript implies what \"done\" looks like."
    )]
    pub acceptance_criteria: Option<Vec<String>>,
    #[schemars(description = "Priority of the card. Omit for normal.")]
    pub priority: Option<PriorityParam>,
    #[schemars(description = "Rough size of the work. Omit when you cannot tell.")]
    pub estimate: Option<EstimateParam>,
    #[schemars(
        description = "Deadline as YYYY-MM-DD. Only when the transcript states one; never invent a date."
    )]
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(rename_all = "snake_case")]
pub enum PriorityParam {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(rename_all = "snake_case")]
pub enum EstimateParam {
    Xs,
    S,
    M,
    L,
    Xl,
}

impl From<PriorityParam> for Priority {
    fn from(param: PriorityParam) -> Self {
        match param {
            PriorityParam::Low => Self::Low,
            PriorityParam::Normal => Self::Normal,
            PriorityParam::High => Self::High,
            PriorityParam::Urgent => Self::Urgent,
        }
    }
}

impl From<EstimateParam> for Estimate {
    fn from(param: EstimateParam) -> Self {
        match param {
            EstimateParam::Xs => Self::Xs,
            EstimateParam::S => Self::S,
            EstimateParam::M => Self::M,
            EstimateParam::L => Self::L,
            EstimateParam::Xl => Self::Xl,
        }
    }
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
    #[schemars(description = "The comment, at most 10000 characters.")]
    pub body: String,
    #[schemars(description = "UUID of the run you are working for, when you were given one.")]
    pub run_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct SearchCardsInput {
    #[schemars(
        description = "Words to look for in the title and the description of the cards, case-insensitive. Omit to list the cards of the column."
    )]
    pub query: Option<String>,
    #[schemars(description = "Only the cards of this column.")]
    pub column: Option<ColumnParam>,
    #[schemars(description = "How many cards to return at most, 20 by default, 50 at most.")]
    pub limit: Option<u32>,
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
            attributes: Attributes {
                acceptance_criteria: AcceptanceCriteria::try_from(
                    input
                        .acceptance_criteria
                        .unwrap_or_default()
                        .into_iter()
                        .map(|text| Criterion { text, done: false })
                        .collect::<Vec<_>>(),
                )?,
                priority: input.priority.map(Priority::from).unwrap_or_default(),
                due_date: input.due_date.as_deref().map(parse_due_date).transpose()?,
                estimate: input.estimate.map(Estimate::from),
                ..Attributes::default()
            },
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
    pub acceptance_criteria: Vec<CriterionView>,
    pub priority: &'static str,
    pub due_date: Option<String>,
    pub estimate: Option<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct CriterionView {
    pub text: String,
    pub done: bool,
}

pub const SEARCH_DEFAULT_LIMIT: u32 = 20;
pub const SEARCH_MAX_LIMIT: u32 = 50;
const SEARCH_DESCRIPTION_CHARS: usize = 300;

#[derive(Debug, Serialize)]
pub struct CardSummary {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub column: &'static str,
    pub version: i32,
}

impl From<&Card> for CardSummary {
    fn from(card: &Card) -> Self {
        Self {
            id: card.id.0,
            title: card.title.as_str().to_owned(),
            description: card
                .description
                .chars()
                .take(SEARCH_DESCRIPTION_CHARS)
                .collect(),
            column: card.column.as_str(),
            version: card.version,
        }
    }
}

pub fn search(cards: &[Card], input: &SearchCardsInput) -> Vec<CardSummary> {
    let words: Vec<String> = input
        .query
        .as_deref()
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_lowercase)
        .collect();
    let column = input.column.map(Column::from);
    let limit = input
        .limit
        .unwrap_or(SEARCH_DEFAULT_LIMIT)
        .clamp(1, SEARCH_MAX_LIMIT) as usize;
    cards
        .iter()
        .filter(|card| column.is_none_or(|column| card.column == column))
        .filter(|card| {
            let text = format!("{} {}", card.title.as_str(), card.description).to_lowercase();
            words.iter().all(|word| text.contains(word.as_str()))
        })
        .take(limit)
        .map(CardSummary::from)
        .collect()
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
            acceptance_criteria: card
                .attributes
                .acceptance_criteria
                .as_slice()
                .iter()
                .map(|criterion| CriterionView {
                    text: criterion.text.clone(),
                    done: criterion.done,
                })
                .collect(),
            priority: card.attributes.priority.as_str(),
            due_date: card.attributes.due_date.map(|date| date.to_string()),
            estimate: card.attributes.estimate.map(Estimate::as_str),
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
        description = "Propose a new card on the Kanban board. Whatever you ask, an agent-created card always lands in the \"proposed\" column: only a human can accept it into \"todo\" or reject it. Give a short actionable title and a description saying what to do and why. Optionally give acceptance_criteria (what must be true when it is done), a priority, an estimate, and a due_date only when the transcript states one. Pass provenance (transcript_id, character offsets start/end, and the exact quote) whenever the card comes from a transcript: the quote is verified against the transcript and a mismatch is refused. Returns the card with its id and its version."
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
        let command = match CreateCard::try_from(input).and_then(|command| {
            if subject.actor.is_some()
                && let Some(date) = command.attributes.due_date
            {
                agent_due_date(date, None)?;
            }
            Ok(command)
        }) {
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
        description = "Search the cards of your organization's board, most recently changed first. Read-only. Give words to look for in the title and the description (all of them must match) and/or a column. Returns up to 50 cards with their id, title, a description excerpt, column and version."
    )]
    #[tracing::instrument(
        skip_all,
        fields(gen_ai.tool.name = "search_cards", organization_id, actor)
    )]
    async fn search_cards(
        &self,
        Parameters(input): Parameters<SearchCardsInput>,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let subject = subject_of(&parts)?;
        into_tool_result(
            self.usecase
                .list_cards(subject)
                .await
                .map(|cards| search(&cards, &input)),
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
            acceptance_criteria: None,
            priority: None,
            estimate: None,
            due_date: None,
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
    fn create_input_carries_criteria_priority_estimate_and_due_date() {
        let mut input = create(None, None);
        input.acceptance_criteria =
            Some(vec!["  Les builds ARM passent ".into(), "Documenté".into()]);
        input.priority = Some(PriorityParam::Urgent);
        input.estimate = Some(EstimateParam::Xl);
        input.due_date = Some("2026-10-31".into());
        let attributes = CreateCard::try_from(input).unwrap().attributes;
        let criteria = attributes.acceptance_criteria.as_slice();
        assert_eq!(criteria.len(), 2);
        assert_eq!(criteria[0].text, "Les builds ARM passent");
        assert!(!criteria[0].done);
        assert_eq!(attributes.priority, Priority::Urgent);
        assert_eq!(attributes.estimate, Some(Estimate::Xl));
        assert_eq!(
            attributes.due_date.map(|date| date.to_string()).as_deref(),
            Some("2026-10-31")
        );
    }

    #[test]
    fn create_input_defaults_to_normal_priority_and_no_details() {
        let attributes = CreateCard::try_from(create(None, None)).unwrap().attributes;
        assert_eq!(attributes, Attributes::default());
    }

    #[test]
    fn an_agent_deadline_in_the_past_or_too_far_ahead_is_refused_with_the_window() {
        for date in ["2001-01-01", "2999-01-01"] {
            let mut input = create(None, None);
            input.due_date = Some(date.into());
            let due = CreateCard::try_from(input)
                .unwrap()
                .attributes
                .due_date
                .unwrap();
            assert!(
                matches!(agent_due_date(due, None), Err(CoreError::Invalid(m)) if m.contains("omit it")),
                "{date}"
            );
        }
    }

    #[test]
    fn create_input_rejects_bad_details() {
        let with = |edit: fn(&mut CreateCardInput)| {
            let mut input = create(None, None);
            edit(&mut input);
            CreateCard::try_from(input)
        };
        assert!(with(|i| i.due_date = Some("vendredi".into())).is_err());
        assert!(with(|i| i.due_date = Some("2026-13-40".into())).is_err());
        assert!(with(|i| i.acceptance_criteria = Some(vec!["  ".into()])).is_err());
        assert!(with(|i| i.acceptance_criteria = Some(vec!["x".into(); 21])).is_err());
        assert!(serde_json::from_value::<PriorityParam>(serde_json::json!("critical")).is_err());
        assert!(serde_json::from_value::<EstimateParam>(serde_json::json!("xxl")).is_err());
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

    fn card(title: &str, description: &str, column: Column) -> Card {
        let now = chrono::Utc::now();
        Card {
            id: CardId::new(),
            organization_id: capitainerie_common::OrganizationId::new(),
            title: capitainerie_core::domain::card::CardTitle::try_from(title.to_owned()).unwrap(),
            description: description.to_owned(),
            column,
            provenance: None,
            created_by_run: None,
            created_by: capitainerie_core::domain::card::Author {
                subject: "u".into(),
                actor: None,
            },
            version: 1,
            created_at: now,
            updated_at: now,
            repository_hint: None,
        }
    }

    fn query(
        text: Option<&str>,
        column: Option<ColumnParam>,
        limit: Option<u32>,
    ) -> SearchCardsInput {
        SearchCardsInput {
            query: text.map(str::to_owned),
            column,
            limit,
        }
    }

    #[test]
    fn search_matches_every_word_in_the_title_or_description_ignoring_case() {
        let cards = [
            card("Ajouter un README", "Pour Acme", Column::Proposed),
            card("Relancer Acme", "Avant vendredi", Column::Todo),
            card("Migrer la CI", "", Column::Todo),
        ];
        let found = search(&cards, &query(Some("acme readme"), None, None));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Ajouter un README");
        assert_eq!(search(&cards, &query(Some("ACME"), None, None)).len(), 2);
        assert_eq!(search(&cards, &query(None, None, None)).len(), 3);
    }

    #[test]
    fn search_filters_by_column_and_bounds_the_result() {
        let cards: Vec<Card> = (0..60)
            .map(|n| card(&format!("Carte {n}"), "", Column::Todo))
            .chain([card("Autre", "", Column::Done)])
            .collect();
        assert_eq!(
            search(&cards, &query(None, Some(ColumnParam::Done), None)).len(),
            1
        );
        assert_eq!(search(&cards, &query(None, None, None)).len(), 20);
        assert_eq!(search(&cards, &query(None, None, Some(500))).len(), 50);
        assert_eq!(search(&cards, &query(None, None, Some(0))).len(), 1);
    }

    #[test]
    fn search_excerpts_long_descriptions() {
        let cards = [card("T", &"é".repeat(1000), Column::Todo)];
        let found = search(&cards, &query(None, None, None));
        assert_eq!(found[0].description.chars().count(), 300);
    }

    #[test]
    fn the_server_exposes_the_search_without_any_other_new_tool() {
        let router = KanbanTools::tool_router();
        let mut names: Vec<_> = router
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "comment",
                "create_card",
                "move_card",
                "search_cards",
                "update_card"
            ]
        );
    }
}
