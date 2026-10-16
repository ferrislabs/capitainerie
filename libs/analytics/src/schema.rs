use std::sync::{Arc, LazyLock};

use arrow_schema::{DataType, Field, Schema, SchemaRef, TimeUnit};

static SCHEMA: LazyLock<SchemaRef> = LazyLock::new(|| {
    Arc::new(Schema::new(vec![
        Field::new("event_id", DataType::Utf8, false),
        Field::new("kind", DataType::Utf8, false),
        Field::new("organization_id", DataType::Utf8, false),
        Field::new("run_id", DataType::Utf8, false),
        Field::new(
            "occurred_at",
            DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
            false,
        ),
        Field::new("initiated_by", DataType::Utf8, true),
        Field::new("seq", DataType::Int32, true),
        Field::new("step_kind", DataType::Utf8, true),
        Field::new("agent", DataType::Utf8, true),
        Field::new("model", DataType::Utf8, true),
        Field::new("input_tokens", DataType::UInt64, true),
        Field::new("output_tokens", DataType::UInt64, true),
        Field::new("cost_micros", DataType::Int64, true),
        Field::new("latency_ms", DataType::Int64, true),
        Field::new("status", DataType::Utf8, true),
        Field::new("failure", DataType::Utf8, true),
    ]))
});

pub fn schema() -> SchemaRef {
    Arc::clone(&SCHEMA)
}
