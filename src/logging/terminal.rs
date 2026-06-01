use crate::error::Result;
use crate::logging::events::{BotLogEvent, LogLevel};
use crate::logging::LogSink;
use async_trait::async_trait;

#[derive(Debug, Default)]
pub struct TerminalSink;

#[async_trait]
impl LogSink for TerminalSink {
    async fn emit(&self, event: &BotLogEvent) -> Result<()> {
        println!("{}", format_terminal_event(event));
        Ok(())
    }
}

fn format_terminal_event(event: &BotLogEvent) -> String {
    let level = match event.level {
        LogLevel::Debug => "debug",
        LogLevel::Info => "info",
        LogLevel::Warn => "warn",
        LogLevel::Error => "error",
    };
    let metadata = terminal_metadata(event);
    if metadata.is_empty() {
        format!("[{level}] [{}] {} {}", event.event, event.message, event.fields)
    } else {
        format!(
            "[{level}] [{}] {} {} {}",
            event.event,
            event.message,
            metadata.join(" "),
            event.fields
        )
    }
}

fn terminal_metadata(event: &BotLogEvent) -> Vec<String> {
    let mut metadata = Vec::new();
    if let Some(tool_name) = &event.tool_name {
        metadata.push(format!("tool_name={tool_name}"));
    }
    if let Some(request_id) = &event.request_id {
        metadata.push(format!("request_id={request_id}"));
    }
    if let Some(trace_id) = &event.trace_id {
        metadata.push(format!("trace_id={trace_id}"));
    }
    if let Some(conversation_id) = &event.conversation_id {
        metadata.push(format!("conversation_id={conversation_id}"));
    }
    metadata
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_tool_events_with_tool_name_metadata() {
        let mut event = BotLogEvent::new(LogLevel::Info, "tool.start", "starting tool");
        event.tool_name = Some("l0_search".to_string());

        let line = format_terminal_event(&event);

        assert!(line.contains("tool_name=l0_search"));
    }
}
