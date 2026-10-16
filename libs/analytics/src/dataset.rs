use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use uuid::Uuid;

use crate::AnalyticsError;

#[derive(Debug, Clone, clap::Args)]
pub struct AnalyticsArgs {
    #[arg(
        long = "analytics-uri",
        env = "ANALYTICS_URI",
        default_value = "./data/analytics"
    )]
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dataset {
    root: PathBuf,
}

impl Dataset {
    pub fn parse(uri: &str) -> Result<Self, AnalyticsError> {
        let path = match uri.strip_prefix("file://") {
            Some(path) => path,
            None if uri.contains("://") => {
                return Err(AnalyticsError::UnsupportedUri(uri.to_owned()));
            }
            None => uri,
        };
        Ok(Self {
            root: PathBuf::from(path),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn organization_dir(&self, organization_id: Uuid) -> PathBuf {
        self.root
            .join("events")
            .join(format!("organization_id={organization_id}"))
    }

    pub fn partition_dir(&self, organization_id: Uuid, date: NaiveDate) -> PathBuf {
        self.organization_dir(organization_id)
            .join(format!("date={date}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_paths_and_file_urls_are_accepted() {
        assert_eq!(
            Dataset::parse("./data/analytics").unwrap().root(),
            Path::new("./data/analytics")
        );
        assert_eq!(
            Dataset::parse("file:///var/analytics").unwrap().root(),
            Path::new("/var/analytics")
        );
    }

    #[test]
    fn object_stores_are_refused_for_now() {
        assert!(matches!(
            Dataset::parse("s3://bucket/analytics"),
            Err(AnalyticsError::UnsupportedUri(_))
        ));
    }

    #[test]
    fn partitions_are_hive_style() {
        let org = Uuid::nil();
        let dir = Dataset::parse("/d")
            .unwrap()
            .partition_dir(org, NaiveDate::from_ymd_opt(2026, 10, 17).unwrap());
        assert_eq!(
            dir,
            PathBuf::from(format!("/d/events/organization_id={org}/date=2026-10-17"))
        );
    }
}
