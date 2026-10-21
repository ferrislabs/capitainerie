use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use capitainerie_core::domain::{
    conversation::turn::{ToolSpec, TurnEvent, TurnSink},
    integration::intervention::IntegratorTools,
};
use serde_json::Value;

use crate::captain::driver::{ToolOutput, Tools};

pub(super) const TIME_BUDGET_SPENT: &str =
    "The time budget of this intervention is spent: stop calling tools and write your report.";
const TIME_BUDGET_SUMMARY: &str = "time budget spent";

pub(super) struct Bounded<'a, T> {
    tools: &'a T,
    started: Instant,
    deadline: Duration,
    refused: AtomicBool,
}

impl<'a, T> Bounded<'a, T> {
    pub(super) fn new(tools: &'a T, deadline: Duration) -> Self {
        Self {
            tools,
            started: Instant::now(),
            deadline,
            refused: AtomicBool::new(false),
        }
    }

    pub(super) fn refused(&self) -> bool {
        self.refused.load(Ordering::SeqCst)
    }
}

impl<T: IntegratorTools> Tools for Bounded<'_, T> {
    fn specs(&self) -> Vec<ToolSpec> {
        self.tools.specs()
    }

    async fn call(&self, name: &str, arguments: &Value) -> ToolOutput {
        if self.started.elapsed() >= self.deadline {
            self.refused.store(true, Ordering::SeqCst);
            return ToolOutput {
                succeeded: false,
                text: TIME_BUDGET_SPENT.to_owned(),
                summary: TIME_BUDGET_SUMMARY.to_owned(),
                card: None,
            };
        }
        let answer = self.tools.call(name, arguments).await;
        ToolOutput {
            succeeded: answer.succeeded,
            text: answer.text,
            summary: answer.summary,
            card: None,
        }
    }
}

pub(super) struct OpenSink;

impl TurnSink for OpenSink {
    async fn emit(&self, _: TurnEvent) -> bool {
        true
    }
}
