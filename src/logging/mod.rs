pub mod events;
pub mod jsonl;
pub mod pubsub;
pub mod redaction;
pub mod terminal;
pub mod websocket;

use crate::error::Result;
use async_trait::async_trait;
use events::BotLogEvent;
use std::sync::Arc;

#[async_trait]
pub trait LogSink: Send + Sync {
    async fn emit(&self, event: &BotLogEvent) -> Result<()>;
}

#[derive(Default)]
pub struct LoggingBus {
    sinks: Vec<Arc<dyn LogSink>>,
}

impl LoggingBus {
    pub fn new(sinks: Vec<Arc<dyn LogSink>>) -> Self {
        Self { sinks }
    }

    pub async fn emit(&self, event: BotLogEvent) {
        for sink in &self.sinks {
            if let Err(error) = sink.emit(&event).await {
                eprintln!("[logging.sink.failure] {error:#}");
            }
        }
    }
}
