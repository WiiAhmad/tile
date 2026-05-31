use crate::error::Result;
use crate::logging::events::BotLogEvent;
use crate::logging::LogSink;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone)]
pub struct JsonlSink {
    path: PathBuf,
}

impl JsonlSink {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait]
impl LogSink for JsonlSink {
    async fn emit(&self, event: &BotLogEvent) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        let line = serde_json::to_string(event)?;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::events::{BotLogEvent, LogLevel};

    #[tokio::test]
    async fn writes_one_json_object_per_line() {
        let path = std::env::temp_dir().join(format!("bot-log-{}.jsonl", uuid::Uuid::new_v4()));
        let sink = JsonlSink::new(&path);
        sink.emit(&BotLogEvent::new(LogLevel::Info, "test.event", "hello"))
            .await
            .unwrap();
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(content.lines().count(), 1);
        let value: serde_json::Value = serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(value["event"], "test.event");
        let _ = tokio::fs::remove_file(path).await;
    }
}
