use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotLogEvent {
    pub id: String,
    pub timestamp_ms: i64,
    pub level: LogLevel,
    pub event: String,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub conversation_id: Option<String>,
    pub telegram_chat_id: Option<i64>,
    pub telegram_user_id: Option<u64>,
    pub tool_name: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub message: String,
    pub fields: serde_json::Value,
}

impl BotLogEvent {
    pub fn new(level: LogLevel, event: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            level,
            event: event.into(),
            request_id: None,
            trace_id: None,
            conversation_id: None,
            telegram_chat_id: None,
            telegram_user_id: None,
            tool_name: None,
            provider: None,
            model: None,
            message: message.into(),
            fields: serde_json::json!({}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_serializes_level_as_snake_case() {
        let event = BotLogEvent::new(LogLevel::Info, "test.event", "hello");
        let json = serde_json::to_value(event).unwrap();
        assert_eq!(json["level"], "info");
        assert_eq!(json["event"], "test.event");
    }
}
