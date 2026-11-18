use capitainerie_common::CoreError;
use capitainerie_core::domain::{
    agent::SecretToken,
    mcp::{CodeExchange, Discovery, RefreshExchange, TokenSet, parse_server_url, ports::McpOAuth},
};
use chrono::{DateTime, Duration, Utc};
use reqwest::header::{ACCEPT, CONTENT_TYPE, WWW_AUTHENTICATE};
use serde_json::{Value, json};
use url::Url;

use super::{McpHttp, refusal};
use crate::guard::{self, Answer};

const DOCUMENT_MAX_BYTES: usize = 256 * 1024;
const RECONNECT: &str = "la connexion a expiré, reconnectez-vous";
const PROTOCOL_VERSION: &str = "2025-06-18";
const CLIENT_NAME: &str = "Capitainerie";
const MAX_SCOPES: usize = 100;
const MAX_CLIENT_ID_CHARS: usize = 512;
const MAX_ERROR_CODE_CHARS: usize = 64;

struct ProtectedResource {
    resource: String,
    issuer: Url,
    scopes: Vec<String>,
}

struct ServerMetadata {
    authorization_endpoint: String,
    token_endpoint: String,
    registration_endpoint: String,
}

#[derive(Debug, PartialEq, Eq)]
enum TokenFailure {
    Refused(Option<String>),
    Unreadable,
}

impl McpHttp {
    async fn fetch_document(&self, what: &str, url: &Url) -> Result<Option<Value>, CoreError> {
        guard::check_url(self.policy, url)
            .await
            .map_err(|failure| refusal(what, &failure))?;
        let answer = guard::send_limited(
            self.client
                .get(url.as_str())
                .header(ACCEPT, "application/json"),
            DOCUMENT_MAX_BYTES,
        )
        .await
        .map_err(|failure| refusal(what, &failure))?;
        Ok(answer.status.is_success().then(|| answer.json()).flatten())
    }

    async fn probe(&self, server: &Url) -> Result<Option<String>, CoreError> {
        guard::check_url(self.policy, server)
            .await
            .map_err(|failure| refusal("le serveur MCP", &failure))?;
        let initialize = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": CLIENT_NAME, "version": "0"},
            },
        });
        let answer = guard::send_limited(
            self.client
                .post(server.as_str())
                .header(ACCEPT, "application/json, text/event-stream")
                .header(CONTENT_TYPE, "application/json")
                .body(initialize.to_string()),
            DOCUMENT_MAX_BYTES,
        )
        .await
        .map_err(|failure| refusal("le serveur MCP", &failure))?;
        Ok(answer
            .headers
            .get_all(WWW_AUTHENTICATE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find_map(resource_metadata_hint))
    }

    async fn protected_resource(&self, server: &Url) -> Result<ProtectedResource, CoreError> {
        let hint = self.probe(server).await?;
        for candidate in protected_resource_candidates(server, hint.as_deref()) {
            if let Some(document) = self
                .fetch_document("les métadonnées OAuth du serveur", &candidate)
                .await?
            {
                return parse_protected_resource(&document, server);
            }
        }
        Err(CoreError::Invalid(
            "ce serveur MCP n'annonce pas de serveur d'autorisation OAuth".into(),
        ))
    }

    async fn server_metadata(&self, issuer: &Url) -> Result<ServerMetadata, CoreError> {
        for candidate in authorization_server_candidates(issuer) {
            if let Some(document) = self
                .fetch_document("les métadonnées du serveur d'autorisation", &candidate)
                .await?
            {
                return parse_server_metadata(&document, issuer);
            }
        }
        Err(CoreError::Invalid(
            "les métadonnées du serveur d'autorisation sont introuvables".into(),
        ))
    }

    async fn token_call(
        &self,
        endpoint: &str,
        form: &[(&str, &str)],
    ) -> Result<TokenSet, CoreError> {
        let target = parse_server_url(endpoint, "l'endpoint de jeton")?;
        guard::check_url(self.policy, &target)
            .await
            .map_err(|failure| refusal("le serveur d'autorisation", &failure))?;
        let answer: Answer = guard::send_limited(
            self.client
                .post(target.as_str())
                .header(ACCEPT, "application/json")
                .form(form),
            DOCUMENT_MAX_BYTES,
        )
        .await
        .map_err(|failure| refusal("le serveur d'autorisation", &failure))?;
        parse_token_response(answer.status.as_u16(), &answer.body, Utc::now()).map_err(|failure| {
            match failure {
                TokenFailure::Refused(Some(code)) if code == "invalid_grant" => {
                    CoreError::Conflict(RECONNECT.into())
                }
                TokenFailure::Refused(Some(code)) => CoreError::Invalid(format!(
                    "le serveur d'autorisation a refusé la demande ({code})"
                )),
                TokenFailure::Refused(None) => {
                    CoreError::Invalid("le serveur d'autorisation a refusé la demande".into())
                }
                TokenFailure::Unreadable => {
                    CoreError::Invalid("la réponse du serveur d'autorisation est illisible".into())
                }
            }
        })
    }
}

impl McpOAuth for McpHttp {
    async fn discover(&self, url: &str) -> Result<Discovery, CoreError> {
        let server = parse_server_url(url, "l'URL du serveur")?;
        let resource = self.protected_resource(&server).await?;
        let metadata = self.server_metadata(&resource.issuer).await?;
        Ok(Discovery {
            resource: resource.resource,
            issuer: resource.issuer.as_str().trim_end_matches('/').to_owned(),
            authorization_endpoint: metadata.authorization_endpoint,
            token_endpoint: metadata.token_endpoint,
            registration_endpoint: metadata.registration_endpoint,
            scopes: resource.scopes,
        })
    }

    async fn register_client(
        &self,
        registration_endpoint: &str,
        redirect_uri: &str,
        scopes: &[String],
    ) -> Result<String, CoreError> {
        let target = parse_server_url(registration_endpoint, "l'endpoint d'enregistrement")?;
        guard::check_url(self.policy, &target)
            .await
            .map_err(|failure| refusal("le serveur d'autorisation", &failure))?;
        let answer = guard::send_limited(
            self.client
                .post(target.as_str())
                .header(ACCEPT, "application/json")
                .header(CONTENT_TYPE, "application/json")
                .body(registration_body(redirect_uri, scopes).to_string()),
            DOCUMENT_MAX_BYTES,
        )
        .await
        .map_err(|failure| refusal("le serveur d'autorisation", &failure))?;
        parse_registration(answer.status.as_u16(), &answer.body)
    }

    async fn exchange_code(&self, request: &CodeExchange<'_>) -> Result<TokenSet, CoreError> {
        self.token_call(
            request.token_endpoint,
            &[
                ("grant_type", "authorization_code"),
                ("code", request.code),
                ("redirect_uri", request.redirect_uri),
                ("client_id", request.client_id),
                ("code_verifier", request.code_verifier.expose()),
                ("resource", request.resource),
            ],
        )
        .await
    }

    async fn refresh(&self, request: &RefreshExchange<'_>) -> Result<TokenSet, CoreError> {
        self.token_call(
            request.token_endpoint,
            &[
                ("grant_type", "refresh_token"),
                ("refresh_token", request.refresh_token.expose()),
                ("client_id", request.client_id),
                ("resource", request.resource),
            ],
        )
        .await
    }
}

fn registration_body(redirect_uri: &str, scopes: &[String]) -> Value {
    let mut body = json!({
        "client_name": CLIENT_NAME,
        "redirect_uris": [redirect_uri],
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
        "token_endpoint_auth_method": "none",
    });
    if !scopes.is_empty() {
        body["scope"] = json!(scopes.join(" "));
    }
    body
}

fn parse_registration(status: u16, body: &[u8]) -> Result<String, CoreError> {
    let document: Option<Value> = serde_json::from_slice(body).ok();
    let client_id = document
        .as_ref()
        .and_then(|document| document["client_id"].as_str())
        .filter(|id| !id.is_empty() && id.chars().count() <= MAX_CLIENT_ID_CHARS);
    match (matches!(status, 200 | 201), client_id) {
        (true, Some(client_id)) => Ok(client_id.to_owned()),
        (true, None) => Err(CoreError::Invalid(
            "le serveur d'autorisation n'a pas renvoyé d'identifiant de client".into(),
        )),
        (false, _) => Err(CoreError::Invalid(format!(
            "l'enregistrement du client a été refusé (statut {status}{})",
            document
                .as_ref()
                .and_then(error_code)
                .map(|code| format!(", {code}"))
                .unwrap_or_default()
        ))),
    }
}

fn error_code(document: &Value) -> Option<String> {
    document["error"]
        .as_str()
        .filter(|code| {
            !code.is_empty()
                && code.chars().count() <= MAX_ERROR_CODE_CHARS
                && code
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
        .map(str::to_owned)
}

fn parse_token_response(
    status: u16,
    body: &[u8],
    now: DateTime<Utc>,
) -> Result<TokenSet, TokenFailure> {
    let document: Value = serde_json::from_slice(body).map_err(|_| {
        if (200..300).contains(&status) {
            TokenFailure::Unreadable
        } else {
            TokenFailure::Refused(None)
        }
    })?;
    if !(200..300).contains(&status) {
        return Err(TokenFailure::Refused(error_code(&document)));
    }
    let access = document["access_token"]
        .as_str()
        .filter(|token| !token.is_empty())
        .ok_or(TokenFailure::Unreadable)?;
    if document["token_type"]
        .as_str()
        .is_some_and(|kind| !kind.eq_ignore_ascii_case("bearer"))
    {
        return Err(TokenFailure::Unreadable);
    }
    let expires_in = document["expires_in"]
        .as_i64()
        .or_else(|| document["expires_in"].as_str()?.parse().ok());
    Ok(TokenSet {
        access_token: SecretToken::new(access),
        refresh_token: document["refresh_token"]
            .as_str()
            .filter(|token| !token.is_empty())
            .map(SecretToken::new),
        expires_at: expires_in
            .filter(|seconds| *seconds > 0)
            .map(|seconds| now + Duration::seconds(seconds)),
        scope: document["scope"].as_str().map(str::to_owned),
    })
}

fn resource_metadata_hint(challenge: &str) -> Option<String> {
    let marker = "resource_metadata=\"";
    let start = challenge.find(marker)? + marker.len();
    let end = challenge[start..].find('"')?;
    Some(challenge[start..start + end].to_owned())
}

fn well_known(origin: &Url, name: &str, path: &str) -> Option<Url> {
    let suffix = path.trim_end_matches('/');
    origin.join(&format!("/.well-known/{name}{suffix}")).ok()
}

fn protected_resource_candidates(server: &Url, hint: Option<&str>) -> Vec<Url> {
    let mut candidates: Vec<Url> = Vec::new();
    let named = [
        hint.and_then(|hint| Url::parse(hint).ok()),
        well_known(server, "oauth-protected-resource", server.path()),
        well_known(server, "oauth-protected-resource", ""),
    ];
    for candidate in named.into_iter().flatten() {
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }
    candidates
}

fn authorization_server_candidates(issuer: &Url) -> Vec<Url> {
    let base = issuer.as_str().trim_end_matches('/');
    let named = [
        Url::parse(&format!("{base}/.well-known/oauth-authorization-server")).ok(),
        well_known(issuer, "oauth-authorization-server", issuer.path()),
        Url::parse(&format!("{base}/.well-known/openid-configuration")).ok(),
    ];
    let mut candidates: Vec<Url> = Vec::new();
    for candidate in named.into_iter().flatten() {
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }
    candidates
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

fn parse_protected_resource(
    document: &Value,
    server: &Url,
) -> Result<ProtectedResource, CoreError> {
    let resource = document["resource"].as_str().unwrap_or(server.as_str());
    let announced = parse_server_url(resource, "la ressource annoncée")?;
    if !same_origin(&announced, server) {
        return Err(CoreError::Invalid(
            "la ressource annoncée n'est pas celle du serveur MCP".into(),
        ));
    }
    let issuer = document["authorization_servers"]
        .as_array()
        .and_then(|servers| servers.first())
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CoreError::Invalid("ce serveur MCP n'annonce pas de serveur d'autorisation".into())
        })?;
    let scopes = document["scopes_supported"]
        .as_array()
        .map(|scopes| {
            scopes
                .iter()
                .filter_map(Value::as_str)
                .filter(|scope| !scope.is_empty() && !scope.contains(char::is_whitespace))
                .take(MAX_SCOPES)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Ok(ProtectedResource {
        resource: resource.to_owned(),
        issuer: parse_server_url(issuer, "le serveur d'autorisation")?,
        scopes,
    })
}

fn parse_server_metadata(document: &Value, issuer: &Url) -> Result<ServerMetadata, CoreError> {
    let announced = document["issuer"].as_str().unwrap_or_default();
    if announced.trim_end_matches('/') != issuer.as_str().trim_end_matches('/') {
        return Err(CoreError::Invalid(
            "l'émetteur annoncé ne correspond pas au serveur d'autorisation".into(),
        ));
    }
    if let Some(methods) = document["code_challenge_methods_supported"].as_array()
        && !methods.iter().any(|method| method == "S256")
    {
        return Err(CoreError::Invalid(
            "le serveur d'autorisation n'accepte pas PKCE S256".into(),
        ));
    }
    let endpoint = |field: &str, what: &str| -> Result<String, CoreError> {
        let value = document[field].as_str().ok_or_else(|| {
            CoreError::Invalid(format!("le serveur d'autorisation n'annonce pas {what}"))
        })?;
        parse_server_url(value, what)?;
        Ok(value.to_owned())
    };
    Ok(ServerMetadata {
        authorization_endpoint: endpoint("authorization_endpoint", "l'endpoint d'autorisation")?,
        token_endpoint: endpoint("token_endpoint", "l'endpoint de jeton")?,
        registration_endpoint: endpoint(
            "registration_endpoint",
            "l'enregistrement dynamique de client",
        )?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(value: &str) -> Url {
        Url::parse(value).unwrap()
    }

    #[test]
    fn the_resource_metadata_comes_from_the_challenge() {
        assert_eq!(
            resource_metadata_hint(
                r#"Bearer resource_metadata="https://demo.mestier.fr/.well-known/oauth-protected-resource""#
            )
            .as_deref(),
            Some("https://demo.mestier.fr/.well-known/oauth-protected-resource")
        );
        assert_eq!(
            resource_metadata_hint(
                r#"Bearer realm="x", resource_metadata="https://a/b", scope="s""#
            )
            .as_deref(),
            Some("https://a/b")
        );
        assert_eq!(resource_metadata_hint("Bearer"), None);
        assert_eq!(
            resource_metadata_hint(r#"Bearer resource_metadata="open"#),
            None
        );
    }

    #[test]
    fn without_a_hint_the_well_known_locations_are_tried_path_first() {
        let candidates: Vec<String> =
            protected_resource_candidates(&url("https://demo.mestier.fr/mcp"), None)
                .iter()
                .map(Url::to_string)
                .collect();
        assert_eq!(
            candidates,
            [
                "https://demo.mestier.fr/.well-known/oauth-protected-resource/mcp",
                "https://demo.mestier.fr/.well-known/oauth-protected-resource",
            ]
        );
        let hinted = protected_resource_candidates(
            &url("https://demo.mestier.fr/mcp"),
            Some("https://demo.mestier.fr/.well-known/oauth-protected-resource"),
        );
        assert_eq!(hinted.len(), 2);
        assert_eq!(
            hinted[0].as_str(),
            "https://demo.mestier.fr/.well-known/oauth-protected-resource"
        );
    }

    #[test]
    fn authorization_server_metadata_falls_back_through_the_known_locations() {
        let candidates: Vec<String> =
            authorization_server_candidates(&url("https://auth.ferrislabs.fr/api/realms/mestier"))
                .iter()
                .map(Url::to_string)
                .collect();
        assert_eq!(
            candidates,
            [
                "https://auth.ferrislabs.fr/api/realms/mestier/.well-known/oauth-authorization-server",
                "https://auth.ferrislabs.fr/.well-known/oauth-authorization-server/api/realms/mestier",
                "https://auth.ferrislabs.fr/api/realms/mestier/.well-known/openid-configuration",
            ]
        );
    }

    #[test]
    fn the_mestier_protected_resource_document_is_understood() {
        let document = json!({
            "resource": "https://demo.mestier.fr/mcp",
            "authorization_servers": ["https://auth.ferrislabs.fr/api/realms/mestier"],
            "scopes_supported": ["mestier.read", "mestier.doe.write"],
        });
        let parsed =
            parse_protected_resource(&document, &url("https://demo.mestier.fr/mcp")).unwrap();
        assert_eq!(parsed.resource, "https://demo.mestier.fr/mcp");
        assert_eq!(
            parsed.issuer.as_str(),
            "https://auth.ferrislabs.fr/api/realms/mestier"
        );
        assert_eq!(parsed.scopes, ["mestier.read", "mestier.doe.write"]);
    }

    #[test]
    fn a_resource_of_another_origin_or_without_an_authorization_server_is_refused() {
        let server = url("https://demo.mestier.fr/mcp");
        assert!(
            parse_protected_resource(
                &json!({"resource": "https://evil.example/mcp", "authorization_servers": ["https://a"]}),
                &server
            )
            .is_err()
        );
        assert!(
            parse_protected_resource(&json!({"resource": "https://demo.mestier.fr/mcp"}), &server)
                .is_err()
        );
        assert!(
            parse_protected_resource(
                &json!({"authorization_servers": ["javascript:alert(1)"]}),
                &server
            )
            .is_err()
        );
    }

    #[test]
    fn server_metadata_needs_the_issuer_the_endpoints_and_pkce() {
        let issuer = url("https://auth.example/realms/m");
        let good = json!({
            "issuer": "https://auth.example/realms/m",
            "authorization_endpoint": "https://auth.example/realms/m/auth",
            "token_endpoint": "https://auth.example/realms/m/token",
            "registration_endpoint": "https://auth.example/realms/m/clients/register",
            "code_challenge_methods_supported": ["S256", "plain"],
        });
        let parsed = parse_server_metadata(&good, &issuer).unwrap();
        assert_eq!(
            parsed.registration_endpoint,
            "https://auth.example/realms/m/clients/register"
        );

        let mut other_issuer = good.clone();
        other_issuer["issuer"] = json!("https://other.example");
        let mut no_pkce = good.clone();
        no_pkce["code_challenge_methods_supported"] = json!(["plain"]);
        let mut no_registration = good.clone();
        no_registration
            .as_object_mut()
            .unwrap()
            .remove("registration_endpoint");
        for bad in [other_issuer, no_pkce, no_registration] {
            assert!(parse_server_metadata(&bad, &issuer).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_registration_asks_for_a_public_pkce_client() {
        let body = registration_body(
            "https://console.example/connectors/oauth/callback",
            &["mestier.read".into(), "mestier.doe.write".into()],
        );
        assert_eq!(body["client_name"], "Capitainerie");
        assert_eq!(
            body["redirect_uris"],
            json!(["https://console.example/connectors/oauth/callback"])
        );
        assert_eq!(
            body["grant_types"],
            json!(["authorization_code", "refresh_token"])
        );
        assert_eq!(body["response_types"], json!(["code"]));
        assert_eq!(body["token_endpoint_auth_method"], "none");
        assert_eq!(body["scope"], "mestier.read mestier.doe.write");
        assert!(
            registration_body("https://c/cb", &[])
                .get("scope")
                .is_none()
        );
    }

    #[test]
    fn a_registration_answer_must_carry_a_client_id() {
        assert_eq!(
            parse_registration(201, br#"{"client_id":"abc","client_secret":"ignored"}"#).unwrap(),
            "abc"
        );
        assert!(parse_registration(201, br#"{}"#).is_err());
        assert!(parse_registration(200, b"not json").is_err());
        let refused = parse_registration(400, br#"{"error":"invalid_redirect_uri"}"#).unwrap_err();
        assert!(refused.to_string().contains("invalid_redirect_uri"));
        let hostile = parse_registration(400, br#"{"error":"<script>"}"#).unwrap_err();
        assert!(!hostile.to_string().contains("script"));
    }

    #[test]
    fn a_token_answer_becomes_a_token_set_with_an_absolute_expiry() {
        let now = Utc::now();
        let tokens = parse_token_response(
            200,
            br#"{"access_token":"at","token_type":"Bearer","expires_in":300,"refresh_token":"rt","scope":"a b"}"#,
            now,
        )
        .unwrap();
        assert_eq!(tokens.access_token.expose(), "at");
        assert_eq!(tokens.refresh_token.unwrap().expose(), "rt");
        assert_eq!(tokens.expires_at, Some(now + Duration::seconds(300)));
        assert_eq!(tokens.scope.as_deref(), Some("a b"));

        let minimal = parse_token_response(200, br#"{"access_token":"at"}"#, now).unwrap();
        assert!(minimal.refresh_token.is_none());
        assert!(minimal.expires_at.is_none());
    }

    #[test]
    fn a_refused_or_unreadable_token_answer_is_an_error_without_the_body() {
        let now = Utc::now();
        assert_eq!(
            parse_token_response(
                400,
                br#"{"error":"invalid_grant","error_description":"secret"}"#,
                now
            )
            .unwrap_err(),
            TokenFailure::Refused(Some("invalid_grant".into()))
        );
        assert_eq!(
            parse_token_response(500, b"<html>", now).unwrap_err(),
            TokenFailure::Refused(None)
        );
        assert_eq!(
            parse_token_response(200, b"<html>", now).unwrap_err(),
            TokenFailure::Unreadable
        );
        assert_eq!(
            parse_token_response(200, br#"{"access_token":"at","token_type":"mac"}"#, now)
                .unwrap_err(),
            TokenFailure::Unreadable
        );
        assert_eq!(
            parse_token_response(200, br#"{"token_type":"bearer"}"#, now).unwrap_err(),
            TokenFailure::Unreadable
        );
    }
}
