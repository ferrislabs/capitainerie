use arrow_array::{
    Array, ArrayRef, Float64Array, Int64Array, RecordBatch, StringArray, TimestampMicrosecondArray,
    UInt64Array,
};
use arrow_schema::{DataType, TimeUnit};
use chrono::{DateTime, Utc};
use datafusion::{
    arrow::compute::cast,
    prelude::{ParquetReadOptions, SessionContext},
    scalar::ScalarValue,
};
use uuid::Uuid;

use crate::{
    AgentRow, AgentsQuery, Analytics, AnalyticsError, CostDimension, CostQuery, CostRow, Dataset,
    record::{RUN_QUEUED, STEP_FINISHED},
    schema,
};

const EVENTS_CTE: &str = "events AS (
    SELECT * FROM (
        SELECT *, ROW_NUMBER() OVER (PARTITION BY event_id ORDER BY occurred_at) AS rn
        FROM raw
        WHERE organization_id = $1
    ) WHERE rn = 1
)";

#[derive(Debug, Clone)]
pub struct DataFusionAnalytics {
    dataset: Dataset,
}

impl DataFusionAnalytics {
    pub fn new(dataset: Dataset) -> Self {
        Self { dataset }
    }

    async fn run(
        &self,
        organization_id: Uuid,
        sql: &str,
        extra: Vec<ScalarValue>,
    ) -> Result<Vec<RecordBatch>, AnalyticsError> {
        let directory = self.dataset.organization_dir(organization_id);
        if !directory.is_dir() {
            return Ok(Vec::new());
        }
        let schema = schema::schema();
        let context = SessionContext::new();
        context
            .register_parquet(
                "raw",
                directory.to_string_lossy().as_ref(),
                ParquetReadOptions::default().schema(&schema),
            )
            .await?;
        let mut params = vec![ScalarValue::Utf8(Some(organization_id.to_string()))];
        params.extend(extra);
        let batches = context
            .sql(sql)
            .await?
            .with_param_values(params)?
            .collect()
            .await?;
        Ok(batches)
    }
}

fn timestamp(value: DateTime<Utc>) -> ScalarValue {
    ScalarValue::TimestampMicrosecond(Some(value.timestamp_micros()), Some("UTC".into()))
}

fn cost_sql(by: CostDimension) -> String {
    let (key, join) = match by {
        CostDimension::Agent => ("COALESCE(s.agent, 'unknown')", ""),
        CostDimension::Model => ("COALESCE(s.model, 'unknown')", ""),
        CostDimension::Day => ("CAST(CAST(s.occurred_at AS DATE) AS VARCHAR)", ""),
        CostDimension::User => (
            "COALESCE(q.initiated_by, 'unknown')",
            "LEFT JOIN queued q ON q.run_id = s.run_id",
        ),
    };
    let order = match by {
        CostDimension::Day => "key ASC",
        _ => "cost_micros DESC, key ASC",
    };
    format!(
        "WITH {EVENTS_CTE},
        steps AS (
            SELECT * FROM events
            WHERE kind = '{STEP_FINISHED}' AND occurred_at >= $2 AND occurred_at < $3
        ),
        queued AS (
            SELECT run_id, MIN(initiated_by) AS initiated_by
            FROM events WHERE kind = '{RUN_QUEUED}' GROUP BY run_id
        )
        SELECT {key} AS key,
            COUNT(DISTINCT s.run_id) AS runs,
            COALESCE(SUM(s.input_tokens), 0) AS input_tokens,
            COALESCE(SUM(s.output_tokens), 0) AS output_tokens,
            COALESCE(SUM(s.cost_micros), 0) AS cost_micros,
            MEDIAN(CAST(s.latency_ms AS DOUBLE)) AS latency_ms_p50
        FROM steps s {join}
        GROUP BY 1
        ORDER BY {order}"
    )
}

fn agents_sql() -> String {
    format!(
        "WITH {EVENTS_CTE}
        SELECT agent,
            COUNT(DISTINCT run_id) FILTER (WHERE occurred_at >= $2 AND occurred_at < $3) AS runs,
            MAX(occurred_at) AS last_seen,
            COALESCE(SUM(input_tokens) FILTER (WHERE occurred_at >= $2 AND occurred_at < $3), 0) AS input_tokens,
            COALESCE(SUM(output_tokens) FILTER (WHERE occurred_at >= $2 AND occurred_at < $3), 0) AS output_tokens,
            COALESCE(SUM(cost_micros) FILTER (WHERE occurred_at >= $2 AND occurred_at < $3), 0) AS cost_micros
        FROM events
        WHERE kind = '{STEP_FINISHED}' AND agent IS NOT NULL
        GROUP BY agent
        ORDER BY agent ASC"
    )
}

fn column(batch: &RecordBatch, index: usize, kind: &DataType) -> Result<ArrayRef, AnalyticsError> {
    let array = batch
        .columns()
        .get(index)
        .ok_or_else(|| AnalyticsError::Result(format!("missing column {index}")))?;
    Ok(cast(array, kind)?)
}

fn typed<T: 'static>(array: &ArrayRef) -> Result<&T, AnalyticsError> {
    array
        .as_any()
        .downcast_ref::<T>()
        .ok_or_else(|| AnalyticsError::Result("unexpected column type".to_owned()))
}

impl Analytics for DataFusionAnalytics {
    #[tracing::instrument(skip_all, fields(organization_id = %query.organization_id), err)]
    async fn costs(&self, query: &CostQuery) -> Result<Vec<CostRow>, AnalyticsError> {
        let batches = self
            .run(
                query.organization_id,
                &cost_sql(query.by),
                vec![timestamp(query.from), timestamp(query.to)],
            )
            .await?;
        let mut rows = Vec::new();
        for batch in &batches {
            let key = column(batch, 0, &DataType::Utf8)?;
            let runs = column(batch, 1, &DataType::UInt64)?;
            let input = column(batch, 2, &DataType::UInt64)?;
            let output = column(batch, 3, &DataType::UInt64)?;
            let cost = column(batch, 4, &DataType::Int64)?;
            let latency = column(batch, 5, &DataType::Float64)?;
            let (key, runs, input, output, cost, latency) = (
                typed::<StringArray>(&key)?,
                typed::<UInt64Array>(&runs)?,
                typed::<UInt64Array>(&input)?,
                typed::<UInt64Array>(&output)?,
                typed::<Int64Array>(&cost)?,
                typed::<Float64Array>(&latency)?,
            );
            for row in 0..batch.num_rows() {
                rows.push(CostRow {
                    key: key.value(row).to_owned(),
                    runs: runs.value(row),
                    input_tokens: input.value(row),
                    output_tokens: output.value(row),
                    cost_micros: cost.value(row),
                    latency_ms_p50: latency.is_valid(row).then(|| latency.value(row)),
                });
            }
        }
        Ok(rows)
    }

    #[tracing::instrument(skip_all, fields(organization_id = %query.organization_id), err)]
    async fn agents(&self, query: &AgentsQuery) -> Result<Vec<AgentRow>, AnalyticsError> {
        let batches = self
            .run(
                query.organization_id,
                &agents_sql(),
                vec![timestamp(query.from), timestamp(query.to)],
            )
            .await?;
        let seen = DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into()));
        let mut rows = Vec::new();
        for batch in &batches {
            let agent = column(batch, 0, &DataType::Utf8)?;
            let runs = column(batch, 1, &DataType::UInt64)?;
            let last_seen = column(batch, 2, &seen)?;
            let input = column(batch, 3, &DataType::UInt64)?;
            let output = column(batch, 4, &DataType::UInt64)?;
            let cost = column(batch, 5, &DataType::Int64)?;
            let (agent, runs, last_seen, input, output, cost) = (
                typed::<StringArray>(&agent)?,
                typed::<UInt64Array>(&runs)?,
                typed::<TimestampMicrosecondArray>(&last_seen)?,
                typed::<UInt64Array>(&input)?,
                typed::<UInt64Array>(&output)?,
                typed::<Int64Array>(&cost)?,
            );
            for row in 0..batch.num_rows() {
                let last_seen = DateTime::from_timestamp_micros(last_seen.value(row))
                    .ok_or_else(|| AnalyticsError::Result("timestamp out of range".to_owned()))?;
                rows.push(AgentRow {
                    agent: agent.value(row).to_owned(),
                    runs: runs.value(row),
                    last_seen,
                    input_tokens: input.value(row),
                    output_tokens: output.value(row),
                    cost_micros: cost.value(row),
                });
            }
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use capitainerie_events::{
        EventEnvelope,
        run::{RunFinished, RunQueued, RunStarted, StepFinished},
    };
    use chrono::TimeZone;
    use uuid::Uuid;

    use super::*;
    use crate::{EventRecord, ParquetEventWriter};

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("capitainerie-analytics-{}", Uuid::now_v7()));
            Self(path)
        }

        fn dataset(&self) -> Dataset {
            Dataset::parse(self.0.to_str().unwrap()).unwrap()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn at(day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, day, hour, 0, 0).unwrap()
    }

    fn envelope<E: capitainerie_events::DomainEvent>(
        event: &E,
        occurred_at: DateTime<Utc>,
    ) -> EventEnvelope {
        EventEnvelope::wrap(event, occurred_at, None).unwrap()
    }

    fn records(envelopes: &[EventEnvelope]) -> Vec<EventRecord> {
        envelopes
            .iter()
            .map(|e| EventRecord::from_envelope(e).unwrap().unwrap())
            .collect()
    }

    fn queued(org: Uuid, run: Uuid, user: &str, when: DateTime<Utc>) -> EventEnvelope {
        envelope(
            &RunQueued {
                run_id: run,
                organization_id: org,
                transcript_id: Some(Uuid::now_v7()),
                card_id: None,
                initiated_by: user.into(),
                pipeline: vec!["extract".into()],
            },
            when,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn step(
        org: Uuid,
        run: Uuid,
        seq: i32,
        agent: &str,
        model: &str,
        tokens: (u64, u64),
        cost: i64,
        latency: i64,
        when: DateTime<Utc>,
    ) -> EventEnvelope {
        envelope(
            &StepFinished {
                run_id: run,
                organization_id: org,
                seq,
                kind: "extract".into(),
                status: "succeeded".into(),
                agent: Some(agent.into()),
                model: Some(model.into()),
                input_tokens: tokens.0,
                output_tokens: tokens.1,
                cost_micros: cost,
                latency_ms: Some(latency),
            },
            when,
        )
    }

    fn query(org: Uuid, by: CostDimension) -> CostQuery {
        CostQuery {
            organization_id: org,
            from: at(1, 0),
            to: at(31, 0),
            by,
        }
    }

    struct Fixture {
        scratch: Scratch,
        org: Uuid,
        analytics: DataFusionAnalytics,
        writer: ParquetEventWriter,
    }

    fn fixture() -> Fixture {
        let scratch = Scratch::new();
        let dataset = scratch.dataset();
        Fixture {
            org: Uuid::now_v7(),
            analytics: DataFusionAnalytics::new(dataset.clone()),
            writer: ParquetEventWriter::new(dataset),
            scratch,
        }
    }

    fn seed(fixture: &Fixture) -> Vec<EventEnvelope> {
        let org = fixture.org;
        let (run_a, run_b) = (Uuid::now_v7(), Uuid::now_v7());
        let events = vec![
            queued(org, run_a, "alice", at(10, 8)),
            envelope(
                &RunStarted {
                    run_id: run_a,
                    organization_id: org,
                    resumed: false,
                },
                at(10, 8),
            ),
            step(
                org,
                run_a,
                0,
                "extractor",
                "qwen",
                (100, 10),
                500,
                100,
                at(10, 9),
            ),
            step(
                org,
                run_a,
                1,
                "writer",
                "qwen",
                (50, 5),
                250,
                300,
                at(10, 10),
            ),
            envelope(
                &RunFinished {
                    run_id: run_a,
                    organization_id: org,
                    status: "succeeded".into(),
                    failure: None,
                },
                at(10, 11),
            ),
            queued(org, run_b, "bob", at(11, 8)),
            step(
                org,
                run_b,
                0,
                "extractor",
                "gpt",
                (200, 20),
                1000,
                200,
                at(11, 9),
            ),
        ];
        fixture.writer.write(&records(&events)).unwrap();
        events
    }

    #[tokio::test]
    async fn costs_by_agent_sum_the_steps() {
        let fixture = fixture();
        seed(&fixture);

        let rows = fixture
            .analytics
            .costs(&query(fixture.org, CostDimension::Agent))
            .await
            .unwrap();

        assert_eq!(
            rows,
            vec![
                CostRow {
                    key: "extractor".into(),
                    runs: 2,
                    input_tokens: 300,
                    output_tokens: 30,
                    cost_micros: 1500,
                    latency_ms_p50: Some(150.0),
                },
                CostRow {
                    key: "writer".into(),
                    runs: 1,
                    input_tokens: 50,
                    output_tokens: 5,
                    cost_micros: 250,
                    latency_ms_p50: Some(300.0),
                },
            ]
        );
    }

    #[tokio::test]
    async fn costs_by_user_join_the_queued_event() {
        let fixture = fixture();
        seed(&fixture);

        let rows = fixture
            .analytics
            .costs(&query(fixture.org, CostDimension::User))
            .await
            .unwrap();

        let summary: Vec<_> = rows
            .iter()
            .map(|r| (r.key.as_str(), r.runs, r.cost_micros))
            .collect();
        assert_eq!(summary, vec![("bob", 1, 1000), ("alice", 1, 750)]);
    }

    #[tokio::test]
    async fn costs_by_model_and_by_day() {
        let fixture = fixture();
        seed(&fixture);

        let by_model = fixture
            .analytics
            .costs(&query(fixture.org, CostDimension::Model))
            .await
            .unwrap();
        let models: Vec<_> = by_model
            .iter()
            .map(|r| (r.key.as_str(), r.cost_micros))
            .collect();
        assert_eq!(models, vec![("gpt", 1000), ("qwen", 750)]);

        let by_day = fixture
            .analytics
            .costs(&query(fixture.org, CostDimension::Day))
            .await
            .unwrap();
        let days: Vec<_> = by_day
            .iter()
            .map(|r| (r.key.as_str(), r.cost_micros))
            .collect();
        assert_eq!(days, vec![("2026-10-10", 750), ("2026-10-11", 1000)]);
    }

    #[tokio::test]
    async fn duplicated_events_count_once() {
        let fixture = fixture();
        let events = seed(&fixture);
        fixture.writer.write(&records(&events)).unwrap();
        fixture.writer.write(&records(&events[2..3])).unwrap();

        let rows = fixture
            .analytics
            .costs(&query(fixture.org, CostDimension::Agent))
            .await
            .unwrap();

        assert_eq!(rows[0].cost_micros, 1500);
        assert_eq!(rows[0].runs, 2);
    }

    #[tokio::test]
    async fn the_window_bounds_the_steps() {
        let fixture = fixture();
        seed(&fixture);

        let rows = fixture
            .analytics
            .costs(&CostQuery {
                from: at(10, 0),
                to: at(11, 0),
                ..query(fixture.org, CostDimension::Agent)
            })
            .await
            .unwrap();

        let keys: Vec<_> = rows
            .iter()
            .map(|r| (r.key.as_str(), r.cost_micros))
            .collect();
        assert_eq!(keys, vec![("extractor", 500), ("writer", 250)]);
    }

    #[tokio::test]
    async fn another_organization_sees_nothing() {
        let fixture = fixture();
        seed(&fixture);
        let other = Uuid::now_v7();
        let other_event = step(
            other,
            Uuid::now_v7(),
            0,
            "intruder",
            "qwen",
            (1, 1),
            9,
            1,
            at(10, 9),
        );
        fixture.writer.write(&records(&[other_event])).unwrap();

        let rows = fixture
            .analytics
            .costs(&query(other, CostDimension::Agent))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, "intruder");

        let mine = fixture
            .analytics
            .costs(&query(fixture.org, CostDimension::Agent))
            .await
            .unwrap();
        assert!(mine.iter().all(|r| r.key != "intruder"));

        let none = fixture
            .analytics
            .costs(&query(Uuid::now_v7(), CostDimension::Agent))
            .await
            .unwrap();
        assert!(none.is_empty());
    }

    #[tokio::test]
    async fn agents_report_last_activity_and_windowed_totals() {
        let fixture = fixture();
        seed(&fixture);
        let old = step(
            fixture.org,
            Uuid::now_v7(),
            0,
            "retired",
            "qwen",
            (9, 9),
            90,
            5,
            at(2, 9),
        );
        fixture.writer.write(&records(&[old])).unwrap();

        let rows = fixture
            .analytics
            .agents(&AgentsQuery {
                organization_id: fixture.org,
                from: at(10, 0),
                to: at(31, 0),
            })
            .await
            .unwrap();

        let summary: Vec<_> = rows
            .iter()
            .map(|r| (r.agent.as_str(), r.runs, r.cost_micros, r.last_seen))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("extractor", 2, 1500, at(11, 9)),
                ("retired", 0, 0, at(2, 9)),
                ("writer", 1, 250, at(10, 10)),
            ]
        );
    }

    #[test]
    fn the_writer_lays_out_hive_partitions() {
        let fixture = fixture();
        seed(&fixture);
        let org_dir = fixture.scratch.dataset().organization_dir(fixture.org);
        assert!(org_dir.join("date=2026-10-10").is_dir());
        assert!(org_dir.join("date=2026-10-11").is_dir());
        let files: Vec<_> = std::fs::read_dir(org_dir.join("date=2026-10-10"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(files.len(), 1);
        assert!(files[0].starts_with("part-") && files[0].ends_with(".parquet"));
    }
}
