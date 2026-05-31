use crate::agents::service::AiService;
use crate::config::Config;
use crate::health::monitor::HealthMonitor;
use crate::l0::model::{L0Record, L0Role, L0Source};
use crate::l0::repository::L0Repository;
use crate::logging::events::{BotLogEvent, LogLevel};
use crate::logging::redaction::snippet;
use crate::logging::LoggingBus;
use crate::telegram::commands::Command;
use crate::telegram::format::{format_health, format_model, format_start};
use crate::types::TelegramMeta;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use uuid::Uuid;

#[derive(Clone)]
pub struct BotState {
    pub config: Config,
    pub l0: Arc<dyn L0Repository>,
    pub logs: Arc<LoggingBus>,
    pub health: Arc<HealthMonitor>,
    pub ai: Arc<AiService>,
}

impl BotState {
    pub fn new(
        config: Config,
        l0: Arc<dyn L0Repository>,
        logs: Arc<LoggingBus>,
        health: Arc<HealthMonitor>,
        ai: Arc<AiService>,
    ) -> Self {
        Self { config, l0, logs, health, ai }
    }
}

pub async fn handle_command(
    bot: Bot,
    msg: Message,
    command: Command,
    state: Arc<BotState>,
) -> ResponseResult<()> {
    log_message_event(&state, "telegram.command.received", &msg, command_name(&command)).await;

    match command {
        Command::Start => {
            bot.send_message(msg.chat.id, format_start()).await?;
        }
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
        }
        Command::Model => {
            bot.send_message(
                msg.chat.id,
                format_model(state.config.ai_provider.as_str(), &state.config.ai_model),
            )
            .await?;
        }
        Command::Health => {
            let report = state.health.latest().await;
            bot.send_message(msg.chat.id, format_health(&report)).await?;
        }
        Command::L0List => {
            let meta = telegram_meta(&msg);
            let records = state
                .l0
                .list(&meta.conversation_id, state.config.l0_search_limit)
                .await;
            bot.send_message(msg.chat.id, format_l0_result(records)).await?;
        }
        Command::L0Search(query) => {
            let meta = telegram_meta(&msg);
            let records = state
                .l0
                .search(&meta.conversation_id, &query, state.config.l0_search_limit)
                .await;
            bot.send_message(msg.chat.id, format_l0_result(records)).await?;
        }
    }

    Ok(())
}

pub async fn handle_text(bot: Bot, msg: Message, state: Arc<BotState>) -> ResponseResult<()> {
    let Some(text) = msg.text().map(ToOwned::to_owned) else {
        return Ok(());
    };

    log_message_event(&state, "telegram.message.received", &msg, "text").await;

    let meta = telegram_meta(&msg);
    if let Err(error) = state.l0.add(user_record(&meta, &text)).await {
        state
            .logs
            .emit(BotLogEvent::new(
                LogLevel::Error,
                "l0.add.failure",
                format!("failed to store user message: {error:#}"),
            ))
            .await;
        bot.send_message(msg.chat.id, "Memory backend error. Please try again.")
            .await?;
        return Ok(());
    }

    let reply = match state.ai.reply(&meta).await {
        Ok(reply) => reply,
        Err(error) => {
            state
                .logs
                .emit(BotLogEvent::new(
                    LogLevel::Error,
                    "ai.request.failure",
                    format!("AI request failed: {error:#}"),
                ))
                .await;
            bot.send_message(msg.chat.id, "AI provider error. Please try again.")
                .await?;
            return Ok(());
        }
    };
    if let Err(error) = state
        .l0
        .add(assistant_record(
            &meta,
            &reply.text,
            &reply.provider,
            &reply.model,
            reply.usage.clone(),
        ))
        .await
    {
        state
            .logs
            .emit(BotLogEvent::new(
                LogLevel::Error,
                "l0.assistant_add.failure",
                format!("failed to store assistant message: {error:#}"),
            ))
            .await;
    }

    bot.send_message(msg.chat.id, reply.text.clone()).await?;
    log_sent_text_event(&state, &msg, &reply.text).await;
    Ok(())
}

fn telegram_meta(msg: &Message) -> TelegramMeta {
    TelegramMeta::from_chat(
        msg.chat.id.0,
        msg.from.as_ref().map(|user| user.id.0),
        Some(msg.id.0),
    )
}

fn user_record(meta: &TelegramMeta, text: &str) -> L0Record {
    L0Record::new_user(
        Uuid::new_v4().to_string(),
        meta.conversation_id.clone(),
        meta.chat_id,
        meta.user_id,
        meta.message_id,
        text.to_string(),
        chrono::Utc::now().timestamp_millis(),
    )
}

fn assistant_record(
    meta: &TelegramMeta,
    text: &str,
    provider: &str,
    model: &str,
    usage: Option<serde_json::Value>,
) -> L0Record {
    L0Record {
        id: Uuid::new_v4().to_string(),
        conversation_id: meta.conversation_id.clone(),
        telegram_chat_id: meta.chat_id,
        telegram_user_id: meta.user_id,
        telegram_message_id: meta.message_id,
        role: L0Role::Assistant,
        content: text.to_string(),
        source: L0Source::AiResponse,
        provider: Some(provider.to_string()),
        model: Some(model.to_string()),
        tool_name: None,
        tool_call_id: None,
        raw_json: usage.map(|usage| serde_json::json!({ "usage": usage })),
        created_at_ms: chrono::Utc::now().timestamp_millis(),
    }
}

fn format_l0_result(records: crate::error::Result<Vec<L0Record>>) -> String {
    match records {
        Ok(records) if records.is_empty() => "No L0 records found.".to_string(),
        Ok(records) => records
            .iter()
            .map(|record| {
                format!(
                    "- {} {:?}: {}",
                    record.id,
                    record.role,
                    snippet(&record.content, 80)
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Err(_) => "Could not read L0 records. Please try again later.".to_string(),
    }
}

async fn log_message_event(state: &BotState, event: &str, msg: &Message, label: &str) {
    let mut log = BotLogEvent::new(LogLevel::Info, event, label.to_string());
    log.conversation_id = Some(format!("telegram:{}", msg.chat.id.0));
    log.telegram_chat_id = Some(msg.chat.id.0);
    log.telegram_user_id = msg.from.as_ref().map(|user| user.id.0);
    log.fields = serde_json::json!({
        "message_id": msg.id.0,
        "text_snippet": msg.text().map(|text| snippet(text, 80)),
    });
    state.logs.emit(log).await;
}

async fn log_sent_text_event(state: &BotState, msg: &Message, text: &str) {
    let mut log = BotLogEvent::new(LogLevel::Info, "telegram.message.sent", "text");
    log.conversation_id = Some(format!("telegram:{}", msg.chat.id.0));
    log.telegram_chat_id = Some(msg.chat.id.0);
    log.telegram_user_id = msg.from.as_ref().map(|user| user.id.0);
    log.fields = serde_json::json!({
        "reply_to_message_id": msg.id.0,
        "text_snippet": snippet(text, 80),
    });
    state.logs.emit(log).await;
}

fn command_name(command: &Command) -> &'static str {
    match command {
        Command::Start => "start",
        Command::Help => "help",
        Command::Model => "model",
        Command::Health => "health",
        Command::L0List => "l0list",
        Command::L0Search(_) => "l0search",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l0::model::L0Record;

    #[test]
    fn formats_empty_l0_result() {
        assert_eq!(format_l0_result(Ok(Vec::new())), "No L0 records found.");
    }

    #[test]
    fn formats_l0_record_with_snippet() {
        let record = L0Record::new_user(
            "id-1".to_string(),
            "telegram:1".to_string(),
            1,
            None,
            None,
            "hello from l0".to_string(),
            1,
        );
        let output = format_l0_result(Ok(vec![record]));
        assert!(output.contains("id-1"));
        assert!(output.contains("hello from l0"));
    }
}
