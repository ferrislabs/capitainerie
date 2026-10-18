use aws_lc_rs::rand;
use capitainerie_common::CoreError;
use capitainerie_core::domain::execution::valid_segment;
use serde_json::json;

use crate::dto::ManifestOwner;

const NAME_MAX_CHARS: usize = 34;
const NAME_PREFIX: &str = "Capitainerie";
const SUFFIX_BYTES: usize = 3;

pub(crate) fn app_name(organization_name: &str, suffix: &str) -> String {
    let room = NAME_MAX_CHARS - NAME_PREFIX.len() - suffix.len() - 2;
    let cleaned: String = organization_name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let words = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = words.chars().take(room).collect();
    match cut.trim() {
        "" => format!("{NAME_PREFIX} {suffix}"),
        name => format!("{NAME_PREFIX} {name} {suffix}"),
    }
}

pub(crate) fn random_suffix() -> Result<String, CoreError> {
    let mut bytes = [0u8; SUFFIX_BYTES];
    rand::fill(&mut bytes)
        .map_err(|_| CoreError::Internal("the system random source failed".into()))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub(crate) fn manifest(console_url: &str, name: &str) -> String {
    let callback = format!("{console_url}/settings/integrations/github/callback");
    json!({
        "name": name,
        "url": console_url,
        "redirect_url": format!("{console_url}/settings/integrations/github/app-callback"),
        "callback_urls": [callback],
        "setup_url": callback,
        "setup_on_update": true,
        "request_oauth_on_install": true,
        "public": false,
        "default_permissions": {
            "contents": "write",
            "pull_requests": "write",
            "metadata": "read",
        },
        "default_events": [],
    })
    .to_string()
}

pub(crate) fn action_url(
    web_base: &str,
    owner: &ManifestOwner,
    state: &str,
) -> Result<String, CoreError> {
    let web_base = web_base.trim_end_matches('/');
    match owner {
        ManifestOwner::User => Ok(format!("{web_base}/settings/apps/new?state={state}")),
        ManifestOwner::Organization { login } if valid_segment(login) => Ok(format!(
            "{web_base}/organizations/{login}/settings/apps/new?state={state}"
        )),
        ManifestOwner::Organization { login } => Err(CoreError::Invalid(format!(
            "{login:?} is not a GitHub organization login"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn the_name_never_exceeds_the_github_limit() {
        for organization in [
            "",
            "Acme",
            "A very long organization name that goes on and on",
            "Équipe Écureuil Étoilée Éclatante",
            "!!!",
        ] {
            let name = app_name(organization, "1a2b3c");
            assert!(name.chars().count() <= 34, "{name}");
            assert!(name.starts_with("Capitainerie "), "{name}");
            assert!(name.ends_with(" 1a2b3c"), "{name}");
        }
        assert_eq!(app_name("Acme", "1a2b3c"), "Capitainerie Acme 1a2b3c");
        assert_eq!(app_name("", "1a2b3c"), "Capitainerie 1a2b3c");
    }

    #[test]
    fn the_suffix_is_six_hex_characters() {
        let suffix = random_suffix().unwrap();
        assert_eq!(suffix.len(), 6);
        assert!(suffix.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn the_manifest_points_back_to_the_console_and_asks_for_least_privilege() {
        let manifest: Value = serde_json::from_str(&manifest(
            "http://localhost:4080",
            "Capitainerie Acme 1a2b3c",
        ))
        .unwrap();
        assert_eq!(manifest["name"], "Capitainerie Acme 1a2b3c");
        assert_eq!(manifest["url"], "http://localhost:4080");
        assert_eq!(
            manifest["redirect_url"],
            "http://localhost:4080/settings/integrations/github/app-callback"
        );
        let callback = "http://localhost:4080/settings/integrations/github/callback";
        assert_eq!(manifest["callback_urls"], json!([callback]));
        assert_eq!(manifest["setup_url"], callback);
        assert_eq!(manifest["setup_on_update"], true);
        assert_eq!(manifest["request_oauth_on_install"], true);
        assert_eq!(manifest["public"], false);
        assert_eq!(
            manifest["default_permissions"],
            json!({"contents": "write", "pull_requests": "write", "metadata": "read"})
        );
        assert_eq!(manifest["default_events"], json!([]));
        // GitHub refuses a hook URL it cannot reach, even inactive: a local
        // console must still be able to create its App.
        assert!(manifest.get("hook_attributes").is_none());
    }

    #[test]
    fn the_form_target_depends_on_who_owns_the_app() {
        assert_eq!(
            action_url("https://github.com/", &ManifestOwner::User, "s.t").unwrap(),
            "https://github.com/settings/apps/new?state=s.t"
        );
        let organization = ManifestOwner::Organization {
            login: "acme-corp".into(),
        };
        assert_eq!(
            action_url("https://github.com", &organization, "s.t").unwrap(),
            "https://github.com/organizations/acme-corp/settings/apps/new?state=s.t"
        );
    }

    #[test]
    fn an_organization_login_cannot_alter_the_path() {
        for login in ["", "a/b", "..", "a?b", "a b", "a#b", "a\\b"] {
            let owner = ManifestOwner::Organization {
                login: login.into(),
            };
            assert!(
                matches!(
                    action_url("https://github.com", &owner, "s"),
                    Err(CoreError::Invalid(_))
                ),
                "{login:?}"
            );
        }
    }
}
