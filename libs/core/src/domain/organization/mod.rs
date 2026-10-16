pub mod ports;
pub mod service;

use capitainerie_common::OrganizationId;
use chrono::{DateTime, Utc};
use thiserror::Error;

/// An organization maps one-to-one to a FerrisKey realm: its humans log in
/// there and its agents are clients of that realm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organization {
    pub id: OrganizationId,
    pub slug: OrganizationSlug,
    pub name: String,
    pub issuer_url: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrganizationSlug(String);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum InvalidSlug {
    #[error("a slug is 1 to 63 characters long")]
    Length,

    #[error("a slug only holds lowercase letters, digits and inner dashes")]
    Charset,
}

impl OrganizationSlug {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for OrganizationSlug {
    type Error = InvalidSlug;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() || value.len() > 63 {
            return Err(InvalidSlug::Length);
        }
        let allowed = value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
        if !allowed || value.starts_with('-') || value.ends_with('-') {
            return Err(InvalidSlug::Charset);
        }
        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_dns_label() {
        let slug = OrganizationSlug::try_from("ferris-labs-2".to_string());
        assert_eq!(
            slug.map(|s| s.as_str().to_owned()),
            Ok("ferris-labs-2".into())
        );
    }

    #[test]
    fn rejects_uppercase_and_edge_dashes() {
        assert_eq!(
            OrganizationSlug::try_from("Ferris".to_string()),
            Err(InvalidSlug::Charset)
        );
        assert_eq!(
            OrganizationSlug::try_from("-ferris".to_string()),
            Err(InvalidSlug::Charset)
        );
        assert_eq!(
            OrganizationSlug::try_from("ferris-".to_string()),
            Err(InvalidSlug::Charset)
        );
    }

    #[test]
    fn rejects_empty_and_too_long() {
        assert_eq!(
            OrganizationSlug::try_from(String::new()),
            Err(InvalidSlug::Length)
        );
        assert_eq!(
            OrganizationSlug::try_from("a".repeat(64)),
            Err(InvalidSlug::Length)
        );
    }
}
