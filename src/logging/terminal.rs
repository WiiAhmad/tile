use crate::error::Result;
use crate::logging::events::{BotLogEvent, LogLevel};
use crate::logging::LogSink;
use async_trait::async_trait;

#[derive(Debug, Default)]
pub struct TerminalSink;

#[async_trait]
impl LogSink for TerminalSink {
    async fn emit(&self, event: &BotLogEvent) -> Result<()> {
        let level = match event.level {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        };
        println!(
            "[{level}] [{}] {} {}",
            event.event,
            event.message,
            event.fields
        );
        Ok(())
    }
}
