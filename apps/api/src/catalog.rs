use capitainerie_runtime_rig::RigModelCatalog;

/// The single place that names the catalog adapter the API serves `/models` with.
pub type ApiCatalog = RigModelCatalog;

pub fn build() -> anyhow::Result<ApiCatalog> {
    Ok(RigModelCatalog::new()?)
}
