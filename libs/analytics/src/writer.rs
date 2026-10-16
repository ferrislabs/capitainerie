use std::{
    collections::BTreeMap,
    fs::{self, File},
    path::PathBuf,
};

use chrono::NaiveDate;
use parquet::{
    arrow::ArrowWriter,
    basic::{Compression, ZstdLevel},
    file::properties::WriterProperties,
};
use uuid::Uuid;

use crate::{AnalyticsError, Dataset, EventRecord, record::to_batch, schema};

#[derive(Debug, Clone)]
pub struct ParquetEventWriter {
    dataset: Dataset,
}

impl ParquetEventWriter {
    pub fn new(dataset: Dataset) -> Self {
        Self { dataset }
    }

    pub fn write(&self, records: &[EventRecord]) -> Result<Vec<PathBuf>, AnalyticsError> {
        let mut partitions: BTreeMap<(Uuid, NaiveDate), Vec<&EventRecord>> = BTreeMap::new();
        for record in records {
            partitions
                .entry((record.organization_id, record.date()))
                .or_default()
                .push(record);
        }
        let properties = WriterProperties::builder()
            .set_compression(Compression::ZSTD(ZstdLevel::default()))
            .build();
        let mut written = Vec::with_capacity(partitions.len());
        for ((organization_id, date), rows) in partitions {
            let directory = self.dataset.partition_dir(organization_id, date);
            fs::create_dir_all(&directory)?;
            let name = format!("part-{}", Uuid::now_v7());
            let staging = directory.join(format!(".{name}.tmp"));
            let target = directory.join(format!("{name}.parquet"));

            let mut writer = ArrowWriter::try_new(
                File::create(&staging)?,
                schema::schema(),
                Some(properties.clone()),
            )?;
            writer.write(&to_batch(&rows)?)?;
            writer.close()?;
            fs::rename(&staging, &target)?;
            written.push(target);
        }
        Ok(written)
    }
}
