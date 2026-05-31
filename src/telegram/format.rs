use crate::health::model::{HealthReport, HealthStatus};

pub fn format_start() -> &'static str {
    "Hello. I am your Telegram AI L0 bot. Send a message to chat, or use /help."
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
    fn formats_health_report() {
        let report = HealthReport::from_checks(vec![HealthCheck {
            name: "l0_round_trip".into(),
            status: HealthStatus::Healthy,
            latency_ms: Some(12),
            message: None,
        }]);
        let formatted = format_health(&report);
        assert!(formatted.contains("Health: healthy"));
        assert!(formatted.contains("- l0_round_trip: healthy (12ms)"));
    }
}
