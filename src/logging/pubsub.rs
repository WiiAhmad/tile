use crate::error::Result;
use crate::logging::events::BotLogEvent;
use crate::logging::LogSink;
use anyhow::Context;
use async_trait::async_trait;
use iii_sdk::{register_worker, InitOptions, TriggerRequest, III};

const PUBLISH_FUNCTION_ID: &str = "publish";

#[derive(Clone)]
pub struct PubsubSink {
    iii: Option<III>,
    topic: String,
    timeout_ms: Option<u64>,
}

impl PubsubSink {
    #[cfg(test)]
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            iii: None,
            topic: topic.into(),
            timeout_ms: None,
        }
    }

    pub fn with_worker(iii_url: impl AsRef<str>, topic: impl Into<String>) -> Self {
        let iii = register_worker(iii_url.as_ref(), InitOptions::default());
        Self::from_client(iii, topic)
    }

    pub fn from_client(iii: III, topic: impl Into<String>) -> Self {
        Self {
            iii: Some(iii),
            topic: topic.into(),
            timeout_ms: None,
        }
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    #[cfg(test)]
    pub fn topic(&self) -> &str {
        &self.topic
    }

    fn publish_request(&self, event: &BotLogEvent) -> Result<TriggerRequest> {
        Ok(TriggerRequest {
            function_id: PUBLISH_FUNCTION_ID.to_string(),
            payload: serde_json::json!({
                "topic": self.topic,
                "data": serde_json::to_value(event)?,
            }),
            action: None,
            timeout_ms: self.timeout_ms,
        })
    }
}

#[async_trait]
impl LogSink for PubsubSink {
    async fn emit(&self, event: &BotLogEvent) -> Result<()> {
        let request = self.publish_request(event)?;
        if let Some(iii) = &self.iii {
            iii.trigger(request)
                .await
                .context("iii pubsub publish failed")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::events::{BotLogEvent, LogLevel};

    #[test]
    fn builds_publish_request_with_topic_and_event_payload() {
        let sink = PubsubSink::new("bot.logs").with_timeout_ms(250);
        let event = BotLogEvent::new(LogLevel::Info, "test.pubsub", "hello");
        let request = sink.publish_request(&event).unwrap();

        assert_eq!(request.function_id, "publish");
        assert_eq!(request.timeout_ms, Some(250));
        assert_eq!(request.payload["topic"], "bot.logs");
        assert_eq!(request.payload["data"]["event"], "test.pubsub");
    }

    #[tokio::test]
    async fn no_client_sink_serializes_but_does_not_require_running_iii() {
        let sink = PubsubSink::new("bot.logs");
        sink.emit(&BotLogEvent::new(LogLevel::Info, "test.noop", "hello"))
            .await
            .unwrap();
        assert_eq!(sink.topic(), "bot.logs");
    }
}
