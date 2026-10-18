use std::collections::HashSet;

use super::claims::{Actor, Claims};
use super::errors::AuthError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub subject: String,
    pub issuer: String,
    pub client_id: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub actor: Option<Actor>,
    pub roles: Vec<String>,
}

impl TryFrom<Claims> for Identity {
    type Error = AuthError;

    fn try_from(claims: Claims) -> Result<Self, Self::Error> {
        let client_id = claims
            .azp
            .ok_or_else(|| AuthError::InvalidToken("missing azp claim".to_owned()))?;
        let display_name =
            display_name(claims.given_name.as_deref(), claims.family_name.as_deref());
        Ok(Self {
            subject: claims.sub,
            issuer: claims.iss,
            client_id,
            username: claims.preferred_username,
            email: claims.email,
            display_name,
            actor: claims.act,
            roles: claims
                .realm_access
                .map(|access| access.roles)
                .unwrap_or_default(),
        })
    }
}

fn display_name(given: Option<&str>, family: Option<&str>) -> Option<String> {
    let full = format!(
        "{} {}",
        given.unwrap_or_default(),
        family.unwrap_or_default()
    );
    let full = full.trim();
    (!full.is_empty()).then(|| full.to_owned())
}

#[derive(Debug, Clone)]
pub struct AcceptedClients(HashSet<String>);

impl AcceptedClients {
    pub fn new(clients: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self(clients.into_iter().map(Into::into).collect())
    }

    pub fn accepts(&self, client_id: &str) -> bool {
        self.0.contains(client_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::claims::RealmAccess;

    fn claims() -> Claims {
        Claims {
            sub: "u1".into(),
            iss: "http://idp/realms/r".into(),
            azp: Some("web".into()),
            exp: 0,
            typ: None,
            preferred_username: Some("jdoe".into()),
            email: None,
            given_name: Some("Jane".into()),
            family_name: Some("Doe".into()),
            act: None,
            realm_access: None,
        }
    }

    #[test]
    fn display_name_is_given_then_family() {
        let identity = Identity::try_from(claims()).unwrap();
        assert_eq!(identity.display_name.as_deref(), Some("Jane Doe"));
        assert_eq!(identity.client_id, "web");
        assert_eq!(identity.username.as_deref(), Some("jdoe"));
    }

    #[test]
    fn display_name_handles_partial_and_empty() {
        let mut only_family = claims();
        only_family.given_name = None;
        let identity = Identity::try_from(only_family).unwrap();
        assert_eq!(identity.display_name.as_deref(), Some("Doe"));

        let mut none = claims();
        none.given_name = Some(String::new());
        none.family_name = None;
        let identity = Identity::try_from(none).unwrap();
        assert_eq!(identity.display_name, None);
    }

    #[test]
    fn roles_come_from_the_realm_access_claim() {
        let mut with_roles = claims();
        with_roles.realm_access = Some(RealmAccess {
            roles: vec!["capitainerie-admin".into(), "offline_access".into()],
        });
        let identity = Identity::try_from(with_roles).unwrap();
        assert_eq!(identity.roles, ["capitainerie-admin", "offline_access"]);
    }

    #[test]
    fn absent_realm_access_means_no_roles() {
        let identity = Identity::try_from(claims()).unwrap();
        assert!(identity.roles.is_empty());
    }

    #[test]
    fn realm_access_parses_from_token_json() {
        let json = r#"{"sub":"u","iss":"i","azp":"web","exp":1,"realm_access":{"roles":["a"]}}"#;
        let parsed: Claims = serde_json::from_str(json).unwrap();
        assert_eq!(Identity::try_from(parsed).unwrap().roles, ["a"]);
        let bare = r#"{"sub":"u","iss":"i","azp":"web","exp":1,"realm_access":{}}"#;
        let parsed: Claims = serde_json::from_str(bare).unwrap();
        assert!(Identity::try_from(parsed).unwrap().roles.is_empty());
    }

    #[test]
    fn missing_azp_is_rejected() {
        let mut c = claims();
        c.azp = None;
        assert!(matches!(
            Identity::try_from(c),
            Err(AuthError::InvalidToken(_))
        ));
    }

    #[test]
    fn accepted_clients_membership() {
        let accepted = AcceptedClients::new(["web", "cli"]);
        assert!(accepted.accepts("web"));
        assert!(accepted.accepts("cli"));
        assert!(!accepted.accepts("other"));
        assert!(!AcceptedClients::new(Vec::<String>::new()).accepts("web"));
    }
}
