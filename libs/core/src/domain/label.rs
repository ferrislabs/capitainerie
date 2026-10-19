use std::collections::HashSet;

use capitainerie_common::CoreError;

pub fn clean_label(value: &str, max: usize, what: &str) -> Result<String, CoreError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(CoreError::Invalid(format!("{what} is empty")));
    }
    if value.chars().count() > max {
        return Err(CoreError::Invalid(format!(
            "{what} exceeds {max} characters"
        )));
    }
    if value
        .chars()
        .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}' | ','))
    {
        return Err(CoreError::Invalid(format!(
            "{what} contains a comma or a control character"
        )));
    }
    Ok(value.to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name(String);

impl Name {
    pub const MAX_CHARS: usize = 120;

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Name {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        clean_label(&value, Self::MAX_CHARS, "the name").map(Self)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Aliases(Vec<String>);

impl Aliases {
    pub const MAX_COUNT: usize = 10;
    pub const MAX_CHARS: usize = 120;

    pub fn as_slice(&self) -> &[String] {
        &self.0
    }
}

impl TryFrom<Vec<String>> for Aliases {
    type Error = CoreError;

    fn try_from(values: Vec<String>) -> Result<Self, Self::Error> {
        let mut aliases = values
            .iter()
            .map(|alias| clean_label(alias, Self::MAX_CHARS, "an alias"))
            .collect::<Result<Vec<_>, _>>()?;
        let mut seen = HashSet::new();
        aliases.retain(|alias| seen.insert(alias.to_lowercase()));
        if aliases.len() > Self::MAX_COUNT {
            return Err(CoreError::Invalid(format!(
                "there are more than {} aliases",
                Self::MAX_COUNT
            )));
        }
        Ok(Self(aliases))
    }
}

pub const NOTES_MAX_CHARS: usize = 5_000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_bounded_in_characters() {
        assert_eq!(
            Name::try_from("  Acme ".to_owned()).unwrap().as_str(),
            "Acme"
        );
        assert!(Name::try_from("é".repeat(Name::MAX_CHARS)).is_ok());
        assert!(Name::try_from("é".repeat(Name::MAX_CHARS + 1)).is_err());
        assert!(Name::try_from("   ".to_owned()).is_err());
    }

    #[test]
    fn names_and_aliases_follow_the_participant_rules() {
        for bad in [
            "A\nB",
            "A\rB",
            "A\u{2028}B",
            "A\u{2029}B",
            "A\u{0}B",
            "A\tB",
            "Dupont, Jean",
        ] {
            assert!(Name::try_from(bad.to_owned()).is_err(), "{bad:?}");
            assert!(Aliases::try_from(vec![bad.to_owned()]).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn aliases_are_trimmed_deduplicated_without_case_and_capped_at_ten() {
        let aliases =
            Aliases::try_from(vec!["Acme SA".into(), " acme sa ".into(), "ACME".into()]).unwrap();
        assert_eq!(aliases.as_slice(), ["Acme SA", "ACME"]);
        let many = |count: usize| (0..count).map(|n| format!("A{n}")).collect::<Vec<_>>();
        assert!(Aliases::try_from(many(Aliases::MAX_COUNT)).is_ok());
        assert!(Aliases::try_from(many(Aliases::MAX_COUNT + 1)).is_err());
        assert!(Aliases::try_from(vec!["x".repeat(Aliases::MAX_CHARS + 1)]).is_err());
        assert!(Aliases::try_from(vec![" ".into()]).is_err());
    }
}
