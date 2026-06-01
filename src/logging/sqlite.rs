use crate::error::Result;
use crate::l0::fts_store::SqliteL0FtsStore;
use crate::logging::events::BotLogEvent;
use crate::logging::LogSink;
use async_trait::async_trait;

#[derive(Clone)]
pub struct SqliteLogSink {
    store: SqliteL0FtsStore,
}

impl SqliteLogSink {
    pub fn new(store: SqliteL0FtsStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl LogSink for SqliteLogSink {
    async fn emit(&self, event: &BotLogEvent) -> Result<()> {
        self.store.add_log_event(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::events::{BotLogEvent, LogLevel};

    #[tokio::test]
    async fn writes_log_event_to_database() {
        let store = SqliteL0FtsStore::in_memory().unwrap();
        let sink = SqliteLogSink::new(store.clone());
        sink.emit(&BotLogEvent::new(LogLevel::Info, "test.sqlite_log", "hello")).await.unwrap();

        let events = store.list_log_events(10).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, "test.sqlite_log");
    }
}
