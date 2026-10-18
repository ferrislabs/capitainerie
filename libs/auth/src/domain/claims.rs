use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub sub: String,
    pub client_id: Option<String>,
    pub act: Option<Box<Actor>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub azp: Option<String>,
    pub exp: i64,
    pub typ: Option<String>,
    pub preferred_username: Option<String>,
    pub email: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub act: Option<Actor>,
    pub realm_access: Option<RealmAccess>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RealmAccess {
    #[serde(default)]
    pub roles: Vec<String>,
}
