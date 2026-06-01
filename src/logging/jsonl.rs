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
        let datestamp = chrono::Utc::now().format("%Y-%m-%d").to_string();
        Self { path: timestamped_jsonl_path(path.into(), &datestamp) }
    }
}

fn timestamped_jsonl_path(mut path: PathBuf, timestamp: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("logs.jsonl");
    path.set_file_name(format!("{timestamp}-{file_name}"));
    path
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

    #[test]
    fn prefixes_jsonl_filename_with_date() {
        let base = PathBuf::from("./logs/logs.jsonl");
        let path = timestamped_jsonl_path(base, "2026-06-01");

        assert_eq!(path, PathBuf::from("./logs/2026-06-01-logs.jsonl"));
    }

    #[test]
    fn preserves_configured_jsonl_filename_after_datetime_prefix() {
        let base = PathBuf::from("./logs/bot-events.jsonl");
        let path = timestamped_jsonl_path(base, "2026-06-01");

        assert_eq!(path, PathBuf::from("./logs/2026-06-01-bot-events.jsonl"));
    }

    #[test]
    fn jsonl_sink_uses_date_only_prefix() {
        let path = std::env::temp_dir().join(format!("bot-log-{}.jsonl", uuid::Uuid::new_v4()));
        let sink = JsonlSink::new(&path);
        let file_name = sink.path.file_name().unwrap().to_string_lossy();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        assert!(file_name.starts_with(&format!("{today}-bot-log-")));
        assert!(!file_name.contains('T'));
    }

    #[tokio::test]
    async fn writes_one_json_object_per_line() {
        let path = std::env::temp_dir().join(format!("bot-log-{}.jsonl", uuid::Uuid::new_v4()));
        let sink = JsonlSink::new(&path);
        sink.emit(&BotLogEvent::new(LogLevel::Info, "test.event", "hello"))
            .await
            .unwrap();
        let content = tokio::fs::read_to_string(&sink.path).await.unwrap();
        assert_eq!(content.lines().count(), 1);
        let value: serde_json::Value = serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(value["event"], "test.event");
        let _ = tokio::fs::remove_file(&sink.path).await;
    }
}
