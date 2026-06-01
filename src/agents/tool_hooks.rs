use crate::error::Result;
use crate::l0::model::{L0Record, L0Role, L0Source};
use crate::l0::repository::L0Repository;
use crate::logging::events::{BotLogEvent, LogLevel};
use crate::logging::LoggingBus;
use crate::types::TelegramMeta;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ToolRuntimeContext {
    pub request_id: String,
    pub trace_id: String,
    pub conversation_id: String,
    pub telegram_chat_id: i64,
    pub telegram_user_id: Option<u64>,
    pub telegram_message_id: Option<i32>,
    pub tool_name: String,
    pub started_at_ms: i64,
}

pub async fn pre_tool_use(
    tool_name: &str,
    raw_args: serde_json::Value,
    runtime: TelegramMeta,
    l0: Arc<dyn L0Repository>,
    logs: Arc<LoggingBus>,
) -> Result<ToolRuntimeContext> {
    let TelegramMeta { conversation_id, chat_id, user_id, message_id } = runtime;
    let ctx = ToolRuntimeContext {
        request_id: Uuid::new_v4().to_string(),
        trace_id: Uuid::new_v4().to_string(),
        conversation_id,
        telegram_chat_id: chat_id,
        telegram_user_id: user_id,
        telegram_message_id: message_id,
        tool_name: tool_name.to_string(),
        started_at_ms: chrono::Utc::now().timestamp_millis(),
    };

    let mut log = BotLogEvent::new(LogLevel::Info, "tool.start", format!("starting tool {tool_name}"));
    log.request_id = Some(ctx.request_id.clone());
    log.trace_id = Some(ctx.trace_id.clone());
    log.conversation_id = Some(ctx.conversation_id.clone());
    log.tool_name = Some(ctx.tool_name.clone());
    logs.emit(log).await;

    l0.add(L0Record {
        id: Uuid::new_v4().to_string(),
        conversation_id: ctx.conversation_id.clone(),
        telegram_chat_id: ctx.telegram_chat_id,
        telegram_user_id: ctx.telegram_user_id,
        telegram_message_id: ctx.telegram_message_id,
        role: L0Role::Tool,
        content: format!("tool call: {tool_name}"),
        source: L0Source::ToolCall,
        provider: None,
        model: None,
        tool_name: Some(tool_name.to_string()),
        tool_call_id: Some(ctx.trace_id.clone()),
        raw_json: Some(raw_args),
        created_at_ms: ctx.started_at_ms,
    }).await?;

    Ok(ctx)
}

pub async fn post_tool_use(
    ctx: &ToolRuntimeContext,
    result: &serde_json::Value,
    l0: Arc<dyn L0Repository>,
    logs: Arc<LoggingBus>,
) -> Result<()> {
    let mut log = BotLogEvent::new(LogLevel::Info, "tool.success", format!("tool {} succeeded", ctx.tool_name));
    log.request_id = Some(ctx.request_id.clone());
    log.trace_id = Some(ctx.trace_id.clone());
    log.conversation_id = Some(ctx.conversation_id.clone());
    log.tool_name = Some(ctx.tool_name.clone());
    log.fields = tool_result_content_fields(result);
    logs.emit(log).await;
    l0.add(L0Record {
        id: Uuid::new_v4().to_string(),
        conversation_id: ctx.conversation_id.clone(),
        telegram_chat_id: ctx.telegram_chat_id,
        telegram_user_id: ctx.telegram_user_id,
        telegram_message_id: ctx.telegram_message_id,
        role: L0Role::Tool,
        content: format!("tool result: {}", ctx.tool_name),
        source: L0Source::ToolResult,
        provider: None,
        model: None,
        tool_name: Some(ctx.tool_name.clone()),
        tool_call_id: Some(ctx.trace_id.clone()),
        raw_json: Some(result.clone()),
        created_at_ms: chrono::Utc::now().timestamp_millis(),
    }).await?;
    Ok(())
}

fn tool_result_content_fields(result: &serde_json::Value) -> serde_json::Value {
    if let Some(content) = result.get("content") {
        serde_json::json!({ "content": content })
    } else if let Some(content) = result.get("result") {
        serde_json::json!({ "content": content })
    } else if let Some(content) = result.get("results") {
        serde_json::json!({ "content": content })
    } else if let Some(content) = result.get("records") {
        serde_json::json!({ "content": content })
    } else {
        serde_json::json!({ "content": result })
    }
}

pub async fn post_tool_failure(
    ctx: &ToolRuntimeContext,
    error: &anyhow::Error,
    l0: Arc<dyn L0Repository>,
    logs: Arc<LoggingBus>,
) -> serde_json::Value {
    let mut log = BotLogEvent::new(LogLevel::Error, "tool.failure", format!("tool {} failed", ctx.tool_name));
    log.request_id = Some(ctx.request_id.clone());
    log.trace_id = Some(ctx.trace_id.clone());
    log.conversation_id = Some(ctx.conversation_id.clone());
    log.tool_name = Some(ctx.tool_name.clone());
    logs.emit(log).await;
    let payload = serde_json::json!({
        "ok": false,
        "error": {
            "code": format!("{}_failed", ctx.tool_name),
            "message": format!("The {} tool failed. Try again later.", ctx.tool_name)
        }
    });

    let _ = l0.add(L0Record {
        id: Uuid::new_v4().to_string(),
        conversation_id: ctx.conversation_id.clone(),
        telegram_chat_id: ctx.telegram_chat_id,
        telegram_user_id: ctx.telegram_user_id,
        telegram_message_id: ctx.telegram_message_id,
        role: L0Role::Tool,
        content: format!("tool failure: {}", ctx.tool_name),
        source: L0Source::ToolFailure,
        provider: None,
        model: None,
        tool_name: Some(ctx.tool_name.clone()),
        tool_call_id: Some(ctx.trace_id.clone()),
        raw_json: Some(serde_json::json!({ "error": error.to_string() })),
        created_at_ms: chrono::Utc::now().timestamp_millis(),
    }).await;

    payload
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l0::memory_repository::MemoryL0Repository;
    use crate::l0::repository::L0Repository;
    use crate::l0::fts_store::SqliteL0FtsStore;
    use crate::logging::sqlite::SqliteLogSink;

    #[tokio::test]
    async fn post_tool_use_logs_content_only_untruncated_result() {
        let l0: Arc<dyn L0Repository> = Arc::new(MemoryL0Repository::new());
        let log_store = SqliteL0FtsStore::in_memory().unwrap();
        let logs = Arc::new(LoggingBus::new(vec![Arc::new(SqliteLogSink::new(log_store.clone()))]));
        let ctx = ToolRuntimeContext {
            request_id: "request-1".to_string(),
            trace_id: "trace-1".to_string(),
            conversation_id: "telegram:1".to_string(),
            telegram_chat_id: 1,
            telegram_user_id: Some(2),
            telegram_message_id: Some(3),
            tool_name: "l0_search".to_string(),
            started_at_ms: 1,
        };
        let full = "x".repeat(2_000);
        let result = serde_json::json!({ "ok": true, "result": full, "debug": { "internal": true } });

        post_tool_use(&ctx, &result, l0, logs).await.unwrap();
        let events = log_store.list_log_events(10).unwrap();
        let success = events.iter().find(|event| event.event == "tool.success").unwrap();

        assert_eq!(success.fields["content"], result["result"]);
        assert!(success.fields.get("result").is_none());
        assert!(success.fields.get("debug").is_none());
        assert_eq!(success.fields["content"].as_str().unwrap().len(), 2_000);
        assert!(!success.fields["content"].as_str().unwrap().contains('…'));
    }
}
