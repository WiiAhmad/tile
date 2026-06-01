use crate::health::model::{HealthReport, HealthStatus};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub const MENU_HELP_CALLBACK: &str = "menu:help";
pub const MENU_HEALTH_CALLBACK: &str = "menu:health";

pub fn format_start() -> &'static str {
    format_menu()
}

pub fn format_menu() -> &'static str {
    "Menu\nChoose an option:"
}

pub fn menu_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("Help", MENU_HELP_CALLBACK),
        InlineKeyboardButton::callback("Health", MENU_HEALTH_CALLBACK),
    ]])
}

pub fn format_model(provider: &str, model: &str) -> String {
    format!("AI provider: {provider}\nModel: {model}")
}

pub fn format_health(report: &HealthReport) -> String {
    let status = match report.overall {
        HealthStatus::Healthy => "healthy",
        HealthStatus::Degraded => "degraded",
        HealthStatus::Unhealthy => "unhealthy",
    };
    let mut lines = vec![format!("Health: {status}")];
    for check in &report.checks {
        let check_status = match check.status {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Unhealthy => "unhealthy",
        };
        let latency = check
            .latency_ms
            .map(|ms| format!(" ({ms}ms)"))
            .unwrap_or_default();
        let message = check
            .message
            .as_ref()
            .map(|msg| format!(" - {msg}"))
            .unwrap_or_default();
        lines.push(format!("- {}: {check_status}{latency}{message}", check.name));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::model::{HealthCheck, HealthReport, HealthStatus};

    #[test]
    fn formats_model() {
        assert_eq!(format_model("anthropic", "claude-sonnet-4-6"), "AI provider: anthropic\nModel: claude-sonnet-4-6");
    }

    #[test]
    fn formats_menu_keyboard_with_help_and_health_buttons() {
        assert_eq!(format_menu(), "Menu\nChoose an option:");
        let keyboard = menu_keyboard();
        assert_eq!(keyboard.inline_keyboard.len(), 1);
        assert_eq!(keyboard.inline_keyboard[0].len(), 2);
        assert_eq!(keyboard.inline_keyboard[0][0].text, "Help");
        assert_eq!(keyboard.inline_keyboard[0][1].text, "Health");
    }

    #[test]
    fn formats_health_report() {
        let report = HealthReport::from_checks(vec![HealthCheck {
            name: "ping".into(),
            status: HealthStatus::Healthy,
            latency_ms: Some(0),
            message: None,
        }]);
        let formatted = format_health(&report);
        assert!(formatted.contains("Health: healthy"));
        assert!(formatted.contains("- ping: healthy (0ms)"));
        assert!(!formatted.contains("ai_provider_config"));
        assert!(!formatted.contains("telegram_config"));
        assert!(!formatted.contains("l0_round_trip"));
    }
}
