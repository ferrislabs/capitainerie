pub mod ports;
pub mod service;

use aws_lc_rs::{
    digest,
    rand::{SecureRandom, SystemRandom},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use capitainerie_common::{
    AgentId, CardId, CoreError, OrganizationId, ProjectId, RunId, SourceEventId, SourceId,
};
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::domain::{
    card::{CardTitle, DESCRIPTION_MAX_CHARS},
    crew::Slug,
};

/// An external input whose events become cards handled by an agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub id: SourceId,
    pub organization_id: OrganizationId,
    pub slug: Slug,
    pub name: String,
    pub kind: SourceKind,
    pub description: String,
    pub enabled: bool,
    pub config: SourceConfig,
    pub webhook_token_hash: TokenHash,
    pub default_agent_id: Option<AgentId>,
    pub project_id: Option<ProjectId>,
    /// Starts the default agent's run on each new card, on behalf of `created_by`.
    pub auto_run: bool,
    pub allow_writes: bool,
    pub archived: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Source {
    /// Whether the public hook may deliver to it.
    pub fn accepts_events(&self) -> bool {
        self.enabled && !self.archived
    }
}

/// A source with its recent activity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceListing {
    pub source: Source,
    pub events_last_24h: i64,
    pub last_event_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Webhook,
}

impl SourceKind {
    pub const ALL: [SourceKind; 1] = [SourceKind::Webhook];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Webhook => "webhook",
        }
    }
}

impl TryFrom<&str> for SourceKind {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == value)
            .ok_or_else(|| CoreError::Invalid(format!("unknown source kind {value}")))
    }
}

/// Free settings the console keeps for the source (patterns, task template…):
/// a JSON object of at most 64 KiB.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceConfig(Value);

impl SourceConfig {
    pub const MAX_BYTES: usize = 64 * 1024;

    pub fn as_value(&self) -> &Value {
        &self.0
    }

    pub fn empty() -> Self {
        Self(Value::Object(Default::default()))
    }
}

impl TryFrom<Value> for SourceConfig {
    type Error = CoreError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if !value.is_object() {
            return Err(CoreError::Invalid(
                "the config must be a JSON object".into(),
            ));
        }
        if value.to_string().len() > Self::MAX_BYTES {
            return Err(CoreError::Invalid(format!(
                "the config exceeds {} bytes",
                Self::MAX_BYTES
            )));
        }
        Ok(Self(value))
    }
}

/// SHA-256 of a webhook token, in lowercase hex: the only form stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenHash(String);

impl TokenHash {
    pub fn of(token: &str) -> Self {
        let hash = digest::digest(&digest::SHA256, token.as_bytes());
        Self(
            hash.as_ref()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
        )
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn from_stored(value: String) -> Result<Self, CoreError> {
        if value.len() != 64 || !value.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')) {
            return Err(CoreError::Internal(
                "a stored token hash is malformed".into(),
            ));
        }
        Ok(Self(value))
    }
}

/// The secret part of a source's webhook URL, shown once.
#[derive(Clone, PartialEq, Eq)]
pub struct WebhookToken(String);

impl std::fmt::Debug for WebhookToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WebhookToken(***)")
    }
}

impl WebhookToken {
    const PREFIX: &str = "src_";
    const RANDOM_BYTES: usize = 32;
    /// Longer than any token issued: a request path beyond it is not worth hashing.
    pub const MAX_CHARS: usize = 64;

    pub fn generate() -> Result<Self, CoreError> {
        let mut bytes = [0u8; Self::RANDOM_BYTES];
        SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| CoreError::Internal("the system random source failed".into()))?;
        Ok(Self(format!(
            "{}{}",
            Self::PREFIX,
            URL_SAFE_NO_PAD.encode(bytes)
        )))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn hash(&self) -> TokenHash {
        TokenHash::of(&self.0)
    }

    /// Whether a presented token could have been issued, checked before any lookup.
    pub fn plausible(token: &str) -> bool {
        token.starts_with(Self::PREFIX)
            && token.len() <= Self::MAX_CHARS
            && token[Self::PREFIX.len()..]
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    }
}

/// A source together with the token just issued for it.
#[derive(Debug, Clone)]
pub struct IssuedSource {
    pub listing: SourceListing,
    pub token: WebhookToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceEventStatus {
    Received,
    CardCreated,
    Ignored,
    Failed,
}

impl SourceEventStatus {
    pub const ALL: [SourceEventStatus; 4] = [
        SourceEventStatus::Received,
        SourceEventStatus::CardCreated,
        SourceEventStatus::Ignored,
        SourceEventStatus::Failed,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Received => "received",
            Self::CardCreated => "card_created",
            Self::Ignored => "ignored",
            Self::Failed => "failed",
        }
    }
}

impl TryFrom<&str> for SourceEventStatus {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
            .ok_or_else(|| CoreError::Invalid(format!("unknown source event status {value}")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEvent {
    pub id: SourceEventId,
    pub organization_id: OrganizationId,
    pub source_id: SourceId,
    pub received_at: DateTime<Utc>,
    pub status: SourceEventStatus,
    /// Untrusted: it only ever reaches a card's description.
    pub payload: Value,
    pub summary: String,
    pub card_id: Option<CardId>,
    pub run_id: Option<RunId>,
    pub error: Option<String>,
}

pub const PAYLOAD_MAX_BYTES: usize = 256 * 1024;
pub const NAME_MAX_CHARS: usize = 120;
pub const DESCRIPTION_MAX_CHARS_SOURCE: usize = 5_000;
pub const ERROR_MAX_CHARS: usize = 2_000;
/// Events a source accepts per minute before the hook answers 429.
pub const EVENTS_PER_MINUTE: i64 = 60;
const TITLE_FIELDS: [&str; 2] = ["title", "subject"];

/// Checks the size of an incoming payload.
pub fn check_payload(payload: &Value) -> Result<(), CoreError> {
    if payload.to_string().len() > PAYLOAD_MAX_BYTES {
        return Err(CoreError::Invalid(format!(
            "the payload exceeds {PAYLOAD_MAX_BYTES} bytes"
        )));
    }
    Ok(())
}

/// The card title of an event: the payload's `title`, `subject` or the first
/// line of its `text`, else a title naming the source.
pub fn card_title(payload: &Value, source_name: &str) -> String {
    let found = TITLE_FIELDS
        .iter()
        .find_map(|field| non_blank(payload.get(field)))
        .or_else(|| {
            non_blank(payload.get("text"))
                .and_then(|text| text.lines().map(str::trim).find(|line| !line.is_empty()))
        });
    let title = match found {
        Some(title) => title.split_whitespace().collect::<Vec<_>>().join(" "),
        None => format!("Événement {source_name}"),
    };
    truncate(&title, CardTitle::MAX_CHARS)
}

fn non_blank(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

/// The card description of an event: the payload, pretty-printed and cut to
/// fit, introduced as data received from outside.
pub fn card_description(payload: &Value, source_name: &str) -> String {
    let header = format!(
        "Événement reçu par la source « {source_name} ». Contenu transmis tel quel par un système externe, non vérifié :\n\n"
    );
    let pretty = serde_json::to_string_pretty(payload).unwrap_or_else(|_| payload.to_string());
    const FENCE_OPEN: &str = "```json\n";
    const FENCE_CLOSE: &str = "\n```";
    const CUT: &str = "\n[…]";
    let frame = header.chars().count()
        + FENCE_OPEN.chars().count()
        + FENCE_CLOSE.chars().count()
        + CUT.chars().count();
    let room = DESCRIPTION_MAX_CHARS.saturating_sub(frame);
    let body = if pretty.chars().count() > room {
        format!("{}{CUT}", truncate(&pretty, room))
    } else {
        pretty
    };
    format!("{header}{FENCE_OPEN}{body}{FENCE_CLOSE}")
}

pub fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use serde_json::json;

    use super::*;

    pub(crate) fn source(organization_id: OrganizationId, slug: &str) -> Source {
        let now = Utc::now();
        Source {
            id: SourceId::new(),
            organization_id,
            slug: Slug::try_from(slug.to_owned()).unwrap(),
            name: slug.to_owned(),
            kind: SourceKind::Webhook,
            description: String::new(),
            enabled: true,
            config: SourceConfig::empty(),
            webhook_token_hash: TokenHash::of("src_x"),
            default_agent_id: None,
            project_id: None,
            auto_run: false,
            allow_writes: false,
            archived: false,
            created_by: "user-1".into(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn a_token_is_random_plausible_and_only_its_hash_is_kept() {
        let first = WebhookToken::generate().unwrap();
        let second = WebhookToken::generate().unwrap();
        assert_ne!(first, second);
        assert!(WebhookToken::plausible(first.expose()));
        assert_eq!(first.hash(), TokenHash::of(first.expose()));
        assert_eq!(first.hash().as_str().len(), 64);
        assert!(!format!("{first:?}").contains(first.expose()));
        assert!(TokenHash::from_stored(first.hash().as_str().to_owned()).is_ok());
        assert!(TokenHash::from_stored("zz".into()).is_err());
        for bad in ["", "abc", "src_a/b", &format!("src_{}", "a".repeat(80))] {
            assert!(!WebhookToken::plausible(bad), "{bad}");
        }
    }

    #[test]
    fn the_title_comes_from_title_subject_or_the_first_line_of_text() {
        assert_eq!(
            card_title(&json!({"title": " Facture  reçue ", "subject": "x"}), "S"),
            "Facture reçue"
        );
        assert_eq!(card_title(&json!({"subject": "Relance"}), "S"), "Relance");
        assert_eq!(
            card_title(
                &json!({"title": " ", "text": "\n\n Première ligne\nSuite"}),
                "S"
            ),
            "Première ligne"
        );
        assert_eq!(
            card_title(&json!({"title": 3}), "Chatwoot"),
            "Événement Chatwoot"
        );
        assert_eq!(card_title(&json!([1, 2]), "Chatwoot"), "Événement Chatwoot");
        let long = card_title(&json!({"title": "é".repeat(500)}), "S");
        assert_eq!(long.chars().count(), CardTitle::MAX_CHARS);
    }

    #[test]
    fn the_description_is_the_fenced_payload_cut_to_fit() {
        let description = card_description(&json!({"a": 1}), "Chatwoot");
        assert!(description.contains("« Chatwoot »"));
        assert!(description.contains("```json\n{\n  \"a\": 1\n}\n```"));

        let big = json!({"text": "x".repeat(50_000)});
        let description = card_description(&big, "S");
        assert!(description.chars().count() <= DESCRIPTION_MAX_CHARS);
        assert!(description.ends_with("[…]\n```"));
    }

    #[test]
    fn the_config_is_a_bounded_object_and_the_payload_is_bounded() {
        assert!(SourceConfig::try_from(json!({"patterns": ["iam.*"]})).is_ok());
        assert!(SourceConfig::try_from(json!([1])).is_err());
        assert!(SourceConfig::try_from(json!({"x": "a".repeat(SourceConfig::MAX_BYTES)})).is_err());
        assert!(check_payload(&json!({"x": "a".repeat(1000)})).is_ok());
        assert!(check_payload(&json!({"x": "a".repeat(PAYLOAD_MAX_BYTES)})).is_err());
    }

    #[test]
    fn kinds_and_statuses_round_trip() {
        for kind in SourceKind::ALL {
            assert_eq!(SourceKind::try_from(kind.as_str()).unwrap(), kind);
        }
        assert!(SourceKind::try_from("rss").is_err());
        for status in SourceEventStatus::ALL {
            assert_eq!(
                SourceEventStatus::try_from(status.as_str()).unwrap(),
                status
            );
        }
    }

    #[test]
    fn only_an_enabled_active_source_accepts_events() {
        let mut source = source(OrganizationId::new(), "chatwoot");
        assert!(source.accepts_events());
        source.enabled = false;
        assert!(!source.accepts_events());
        source.enabled = true;
        source.archived = true;
        assert!(!source.accepts_events());
    }
}
