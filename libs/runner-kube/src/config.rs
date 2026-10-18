use std::collections::BTreeMap;

use k8s_openapi::{
    api::core::v1::ResourceRequirements, apimachinery::pkg::api::resource::Quantity,
};

pub const DEFAULT_NAMESPACE: &str = "capitainerie-tasks";
pub const DEFAULT_TTL_SECONDS: i32 = 600;

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

fn default_resources() -> ResourceRequirements {
    let quantities = |cpu: &str, memory: &str| {
        BTreeMap::from([
            ("cpu".to_owned(), Quantity(cpu.to_owned())),
            ("memory".to_owned(), Quantity(memory.to_owned())),
        ])
    };
    ResourceRequirements {
        requests: Some(quantities("250m", "512Mi")),
        limits: Some(quantities("2", "2Gi")),
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
        assert_eq!(config.ttl_seconds_after_finished, 600);
        assert_eq!(config.proxy_base_url, "http://proxy:4130");
    }
}
