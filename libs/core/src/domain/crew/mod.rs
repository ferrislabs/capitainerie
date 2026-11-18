pub mod ports;
pub mod service;

use capitainerie_common::{AgentId, CoreError, OrganizationId};
use chrono::{DateTime, Utc};

use crate::domain::label::clean_label;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrewAgent {
    pub id: AgentId,
    pub organization_id: OrganizationId,
    pub slug: Slug,
    pub name: String,
    pub role: String,
    pub description: String,
    pub model: String,
    pub status: AgentStatus,
    pub hue: Hue,
    pub reports_to: Option<AgentId>,
    pub archived: bool,
    pub instructions: String,
    pub connector_grants: ConnectorGrants,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Idle,
    Running,
}

impl AgentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
        }
    }
}

impl TryFrom<&str> for AgentStatus {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "idle" => Ok(Self::Idle),
            "running" => Ok(Self::Running),
            other => Err(CoreError::Invalid(format!("unknown agent status {other}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slug(String);

impl Slug {
    pub const MAX_CHARS: usize = 60;

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Slug {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let slug = value.trim();
        let kebab = !slug.is_empty()
            && slug.split('-').all(|part| {
                !part.is_empty()
                    && part
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            });
        if !kebab || slug.len() > Self::MAX_CHARS {
            return Err(CoreError::Invalid(format!(
                "the slug must be kebab-case (a-z, 0-9 and dashes) of at most {} characters",
                Self::MAX_CHARS
            )));
        }
        Ok(Self(slug.to_owned()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hue(i16);

impl Hue {
    pub const MAX: i16 = 360;

    pub fn get(self) -> i16 {
        self.0
    }
}

impl TryFrom<i16> for Hue {
    type Error = CoreError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        if (0..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(CoreError::Invalid(format!(
                "the hue is between 0 and {}",
                Self::MAX
            )))
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectorGrants(Vec<String>);

impl ConnectorGrants {
    pub const MAX_COUNT: usize = 50;
    pub const MAX_CHARS: usize = 60;

    pub fn as_slice(&self) -> &[String] {
        &self.0
    }
}

impl TryFrom<Vec<String>> for ConnectorGrants {
    type Error = CoreError;

    fn try_from(values: Vec<String>) -> Result<Self, Self::Error> {
        let mut grants = Vec::with_capacity(values.len());
        for value in &values {
            let grant = clean_label(value, Self::MAX_CHARS, "a connector name")?;
            if !grants.contains(&grant) {
                grants.push(grant);
            }
        }
        if grants.len() > Self::MAX_COUNT {
            return Err(CoreError::Invalid(format!(
                "there are more than {} connector grants",
                Self::MAX_COUNT
            )));
        }
        Ok(Self(grants))
    }
}

pub const ROLE_MAX_CHARS: usize = 120;
pub const MODEL_MAX_CHARS: usize = 200;
pub const DESCRIPTION_MAX_CHARS: usize = 5_000;
pub const INSTRUCTIONS_MAX_CHARS: usize = 20_000;

/// Whether making `parent` the manager of `agent` would close a loop.
pub fn closes_a_loop(agents: &[CrewAgent], agent: AgentId, parent: AgentId) -> bool {
    let mut current = Some(parent);
    for _ in 0..=agents.len() {
        match current {
            Some(id) if id == agent => return true,
            Some(id) => {
                current = agents
                    .iter()
                    .find(|candidate| candidate.id == id)
                    .and_then(|candidate| candidate.reports_to);
            }
            None => return false,
        }
    }
    true
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn agent(organization_id: OrganizationId, slug: &str) -> CrewAgent {
        let now = Utc::now();
        CrewAgent {
            id: AgentId::new(),
            organization_id,
            slug: Slug::try_from(slug.to_owned()).unwrap(),
            name: slug.to_owned(),
            role: String::new(),
            description: String::new(),
            model: String::new(),
            status: AgentStatus::Idle,
            hue: Hue::try_from(0).unwrap(),
            reports_to: None,
            archived: false,
            instructions: String::new(),
            connector_grants: ConnectorGrants::default(),
            created_by: "user-1".into(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn slugs_are_kebab_case() {
        for good in ["dev", "tech-lead", "agent-2"] {
            assert!(Slug::try_from(good.to_owned()).is_ok(), "{good}");
        }
        for bad in ["", "Dev", "a--b", "-a", "a-", "a b", "é", &"a".repeat(61)] {
            assert!(Slug::try_from(bad.to_owned()).is_err(), "{bad}");
        }
    }

    #[test]
    fn hues_stay_on_the_wheel() {
        assert!(Hue::try_from(0).is_ok());
        assert!(Hue::try_from(360).is_ok());
        assert!(Hue::try_from(361).is_err());
        assert!(Hue::try_from(-1).is_err());
    }

    #[test]
    fn grants_are_cleaned_and_deduplicated() {
        let grants =
            ConnectorGrants::try_from(vec![" github ".into(), "github".into(), "odoo".into()])
                .unwrap();
        assert_eq!(grants.as_slice(), ["github", "odoo"]);
        assert!(ConnectorGrants::try_from(vec!["a,b".into()]).is_err());
    }

    #[test]
    fn a_loop_in_the_reporting_line_is_detected() {
        let organization_id = OrganizationId::new();
        let mut boss = agent(organization_id, "boss");
        let mut lead = agent(organization_id, "lead");
        let dev = agent(organization_id, "dev");
        lead.reports_to = Some(boss.id);
        let mut dev = dev;
        dev.reports_to = Some(lead.id);
        let all = [boss.clone(), lead.clone(), dev.clone()];
        assert!(closes_a_loop(&all, boss.id, dev.id));
        assert!(closes_a_loop(&all, boss.id, boss.id));
        assert!(!closes_a_loop(&all, dev.id, boss.id));
        boss.reports_to = Some(dev.id);
        let corrupt = [boss.clone(), lead, dev];
        assert!(closes_a_loop(&corrupt, AgentId::new(), boss.id));
    }
}
