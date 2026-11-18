pub mod ports;
pub mod service;

use capitainerie_common::{CoreError, OrganizationId, SkillId};
use chrono::{DateTime, Utc};

use crate::domain::crew::Slug;

/// An entry of the organization's catalog: instructions an agent follows when
/// its `skills` list names the slug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    pub id: SkillId,
    pub organization_id: OrganizationId,
    pub slug: Slug,
    pub name: String,
    pub description: String,
    pub content: String,
    pub source: SkillSource,
    pub archived: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A skill with the number of active agents whose `skills` name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogSkill {
    pub skill: Skill,
    pub used_by: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    Manual,
    Url(String),
}

impl SkillSource {
    pub const MAX_CHARS: usize = 2_000;
    const MANUAL: &str = "manual";

    pub fn as_str(&self) -> &str {
        match self {
            Self::Manual => Self::MANUAL,
            Self::Url(url) => url,
        }
    }
}

impl TryFrom<String> for SkillSource {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value = value.trim();
        if value.is_empty() || value == Self::MANUAL {
            return Ok(Self::Manual);
        }
        let parsed = url::Url::parse(value).map_err(|_| invalid_source())?;
        if !matches!(parsed.scheme(), "http" | "https") || value.chars().count() > Self::MAX_CHARS {
            return Err(invalid_source());
        }
        Ok(Self::Url(value.to_owned()))
    }
}

fn invalid_source() -> CoreError {
    CoreError::Invalid(format!(
        "the source is \"manual\" or an http(s) URL of at most {} characters",
        SkillSource::MAX_CHARS
    ))
}

pub const DESCRIPTION_MAX_CHARS: usize = 5_000;
pub const CONTENT_MAX_CHARS: usize = 100_000;
/// What a run's system prompt takes from the agent's skills, all together.
pub const PROMPT_SKILLS_MAX_CHARS: usize = 40_000;

/// The skills section of an agent's system prompt, in the order the agent
/// lists them; the whole stays under `max` characters, the last skills being
/// cut first.
pub fn skills_prompt(skills: &[Skill], max: usize) -> Option<String> {
    let mut sections = Vec::new();
    let mut left = max;
    for skill in skills {
        let content = skill.content.trim();
        if content.is_empty() {
            continue;
        }
        let header = format!("## {} ({})\n", skill.name, skill.slug.as_str());
        let header_chars = header.chars().count();
        if left <= header_chars {
            break;
        }
        let room = left - header_chars;
        let body: String = content.chars().take(room).collect();
        let cut = body.chars().count() < content.chars().count();
        left -= header_chars + body.chars().count();
        sections.push(format!("{header}{body}{}", if cut { "\n[…]" } else { "" }));
        if cut {
            break;
        }
    }
    if sections.is_empty() {
        return None;
    }
    Some(format!(
        "# Compétences\n\nTu disposes des compétences suivantes ; applique-les quand elles concernent la tâche.\n\n{}",
        sections.join("\n\n")
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn skill(organization_id: OrganizationId, slug: &str, content: &str) -> Skill {
        let now = Utc::now();
        Skill {
            id: SkillId::new(),
            organization_id,
            slug: Slug::try_from(slug.to_owned()).unwrap(),
            name: slug.to_owned(),
            description: String::new(),
            content: content.to_owned(),
            source: SkillSource::Manual,
            archived: false,
            created_by: "user-1".into(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn a_source_is_manual_or_a_web_url() {
        assert_eq!(
            SkillSource::try_from(String::new()).unwrap(),
            SkillSource::Manual
        );
        assert_eq!(
            SkillSource::try_from(" manual ".to_owned()).unwrap(),
            SkillSource::Manual
        );
        let url = SkillSource::try_from("https://example.com/skill.md".to_owned()).unwrap();
        assert_eq!(url.as_str(), "https://example.com/skill.md");
        for bad in [
            "ftp://example.com/a".to_owned(),
            "not a url".to_owned(),
            format!("https://example.com/{}", "a".repeat(SkillSource::MAX_CHARS)),
        ] {
            assert!(SkillSource::try_from(bad.clone()).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_prompt_lists_skills_in_order_and_skips_empty_ones() {
        let organization_id = OrganizationId::new();
        let prompt = skills_prompt(
            &[
                skill(organization_id, "review", "Relis tout."),
                skill(organization_id, "empty", "  "),
                skill(organization_id, "deploy", "Déploie."),
            ],
            PROMPT_SKILLS_MAX_CHARS,
        )
        .unwrap();
        let review = prompt.find("## review (review)\nRelis tout.").unwrap();
        let deploy = prompt.find("## deploy (deploy)\nDéploie.").unwrap();
        assert!(review < deploy);
        assert!(!prompt.contains("empty"));
        assert!(skills_prompt(&[], PROMPT_SKILLS_MAX_CHARS).is_none());
    }

    #[test]
    fn the_prompt_is_bounded() {
        let organization_id = OrganizationId::new();
        let skills = [
            skill(organization_id, "big", &"é".repeat(500)),
            skill(organization_id, "next", "jamais lu"),
        ];
        let prompt = skills_prompt(&skills, 100).unwrap();
        assert!(prompt.contains("[…]"));
        assert!(!prompt.contains("jamais lu"));
        let body: usize = prompt.matches('é').count();
        assert!(body < 100, "{body}");
    }
}
