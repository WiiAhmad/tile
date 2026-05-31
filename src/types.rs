use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TelegramMeta {
    pub conversation_id: String,
    pub chat_id: i64,
    pub user_id: Option<u64>,
    pub message_id: Option<i32>,
}

impl TelegramMeta {
    pub fn from_chat(chat_id: i64, user_id: Option<u64>, message_id: Option<i32>) -> Self {
        Self {
            conversation_id: format!("telegram:{chat_id}"),
            chat_id,
            user_id,
            message_id,
        }
    }
}
