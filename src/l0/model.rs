use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum L0Role {
    System,
    User,
    Assistant,
    Tool,
    Telegram,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum L0Source {
    TelegramUpdate,
    AiRequest,
    AiResponse,
    ToolCall,
    ToolResult,
    ToolFailure,
    LogEvent,
    HealthCheck,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct L0Record {
    pub id: String,
    pub conversation_id: String,
    pub telegram_chat_id: i64,
    pub telegram_user_id: Option<u64>,
    pub telegram_message_id: Option<i32>,
    pub role: L0Role,
    pub content: String,
    pub source: L0Source,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub tool_name: Option<String>,
    pub tool_call_id: Option<String>,
    pub raw_json: Option<serde_json::Value>,
    pub created_at_ms: i64,
}

impl L0Record {
    pub fn new_user(
        id: String,
        conversation_id: String,
        telegram_chat_id: i64,
        telegram_user_id: Option<u64>,
        telegram_message_id: Option<i32>,
        content: String,
        created_at_ms: i64,
    ) -> Self {
        Self {
            id,
            conversation_id,
            telegram_chat_id,
            telegram_user_id,
            telegram_message_id,
            role: L0Role::User,
            content,
            source: L0Source::TelegramUpdate,
            provider: None,
            model: None,
            tool_name: None,
            tool_call_id: None,
            raw_json: None,
            created_at_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_role_as_snake_case() {
        let role = serde_json::to_string(&L0Role::Assistant).unwrap();
        assert_eq!(role, "\"assistant\"");
    }

    #[test]
    fn creates_user_record() {
        let record = L0Record::new_user(
            "id-1".to_string(),
            "telegram:42".to_string(),
            42,
            Some(7),
            Some(9),
            "hello".to_string(),
            1000,
        );
        assert_eq!(record.role, L0Role::User);
        assert_eq!(record.source, L0Source::TelegramUpdate);
        assert_eq!(record.conversation_id, "telegram:42");
    }
}
