use std::collections::BTreeMap;

use k8s_openapi::{
    api::core::v1::ResourceRequirements, apimachinery::pkg::api::resource::Quantity,
};

pub const DEFAULT_NAMESPACE: &str = "capitainerie-tasks";
pub const DEFAULT_TTL_SECONDS: i32 = 1800;
pub const EPHEMERAL_STORAGE: &str = "6Gi";

#[derive(Debug, Clone)]
pub struct KubeRunnerConfig {
    pub namespace: String,
    pub image: String,
    pub proxy_base_url: String,
    pub resources: ResourceRequirements,
    pub ttl_seconds_after_finished: i32,
    pub command: Option<Vec<String>>,
}

impl KubeRunnerConfig {
    /// Host and port of the credentials proxy, which the pod can only reach
    /// through a pinned address.
    pub fn proxy_endpoint(&self) -> Option<(String, u16)> {
        let (scheme, rest) = self.proxy_base_url.split_once("://")?;
        let authority = rest.split('/').next()?;
        let authority = authority.rsplit('@').next()?;
        let default_port = if scheme == "https" { 443 } else { 80 };
        match authority.rsplit_once(':') {
            Some((host, port)) if !host.is_empty() && !host.ends_with(':') => {
                Some((host.to_owned(), port.parse().ok()?))
            }
            _ if !authority.is_empty() => Some((authority.to_owned(), default_port)),
            _ => None,
        }
    }

    pub fn new(image: impl Into<String>, proxy_base_url: impl Into<String>) -> Self {
        Self {
            namespace: DEFAULT_NAMESPACE.to_owned(),
            image: image.into(),
            proxy_base_url: proxy_base_url.into().trim_end_matches('/').to_owned(),
            resources: default_resources(),
            ttl_seconds_after_finished: DEFAULT_TTL_SECONDS,
            command: None,
        }
    }
}

fn with_storage(mut limits: BTreeMap<String, Quantity>) -> BTreeMap<String, Quantity> {
    limits.insert(
        "ephemeral-storage".to_owned(),
        Quantity(EPHEMERAL_STORAGE.to_owned()),
    );
    limits
}

fn default_resources() -> ResourceRequirements {
    let quantities = |cpu: &str, memory: &str| {
        BTreeMap::from([
            ("cpu".to_owned(), Quantity(cpu.to_owned())),
            ("memory".to_owned(), Quantity(memory.to_owned())),
        ])
    };
    ResourceRequirements {
        requests: Some(quantities("250m", "512Mi")),
        limits: Some(with_storage(quantities("2", "2Gi"))),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_spec() {
        let config = KubeRunnerConfig::new("capitainerie-executor:dev", "http://proxy:4130/");
        assert_eq!(config.namespace, "capitainerie-tasks");
        assert_eq!(config.ttl_seconds_after_finished, 1800);
        assert_eq!(config.proxy_base_url, "http://proxy:4130");
        let limits = config.resources.limits.unwrap();
        assert_eq!(limits["ephemeral-storage"].0, "6Gi");
    }

    #[test]
    fn the_proxy_endpoint_is_read_from_its_url() {
        let endpoint = |url: &str| KubeRunnerConfig::new("i", url).proxy_endpoint();
        assert_eq!(
            endpoint("http://capitainerie-egress-proxy.capitainerie.svc:4130"),
            Some((
                "capitainerie-egress-proxy.capitainerie.svc".to_owned(),
                4130
            ))
        );
        assert_eq!(
            endpoint("http://proxy/path"),
            Some(("proxy".to_owned(), 80))
        );
        assert_eq!(endpoint("https://proxy"), Some(("proxy".to_owned(), 443)));
        assert_eq!(endpoint("proxy:4130"), None);
        assert_eq!(endpoint("http://"), None);
    }
}
