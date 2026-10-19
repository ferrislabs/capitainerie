use capitainerie_core::domain::client::links::Change;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Patch<T> {
    #[default]
    Absent,
    Null,
    Value(T),
}

impl<T> Patch<T> {
    pub fn change(self) -> Change<T> {
        match self {
            Self::Absent => Change::Keep,
            Self::Null => Change::Clear,
            Self::Value(value) => Change::Set(value),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(value) => Self::Value(value),
            None => Self::Null,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    #[derive(Deserialize)]
    struct Body {
        #[serde(default)]
        field: Patch<u32>,
    }

    fn parse(value: serde_json::Value) -> Patch<u32> {
        serde_json::from_value::<Body>(value).unwrap().field
    }

    #[test]
    fn absent_null_and_a_value_are_three_different_things() {
        assert_eq!(parse(json!({})), Patch::Absent);
        assert_eq!(parse(json!({"field": null})), Patch::Null);
        assert_eq!(parse(json!({"field": 7})), Patch::Value(7));
        assert_eq!(parse(json!({})).change(), Change::Keep);
        assert_eq!(parse(json!({"field": null})).change(), Change::Clear);
        assert_eq!(parse(json!({"field": 7})).change(), Change::Set(7));
    }

    #[test]
    fn a_wrong_type_is_an_error() {
        assert!(serde_json::from_value::<Body>(json!({"field": "x"})).is_err());
    }
}
