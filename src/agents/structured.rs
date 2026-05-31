use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct TelegramAssistantOutput {
    pub reply: String,
    pub should_store_memory: bool,
    pub memory_tags: Vec<String>,
}

impl TelegramAssistantOutput {
    #[cfg(test)]
    pub fn fallback(reply: impl Into<String>) -> Self {
        Self {
            reply: reply.into(),
            should_store_memory: false,
            memory_tags: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_has_reply_and_no_memory_tags() {
        let output = TelegramAssistantOutput::fallback("hello");
        assert_eq!(output.reply, "hello");
        assert!(!output.should_store_memory);
        assert!(output.memory_tags.is_empty());
    }

    #[test]
    fn schema_can_be_generated() {
        let schema = schemars::schema_for!(TelegramAssistantOutput);
        let value = serde_json::to_value(schema).unwrap();
        assert!(value.to_string().contains("reply"));
    }
}
