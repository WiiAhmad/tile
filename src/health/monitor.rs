use crate::config::Config;
use crate::health::checks::{check_iii_pubsub, check_ping};
use crate::health::model::{HealthCheck, HealthReport, HealthStatus};
use crate::logging::events::{BotLogEvent, LogLevel};
use crate::logging::LoggingBus;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct HealthMonitor {
    config: Config,
    logs: Arc<LoggingBus>,
    latest: Arc<RwLock<HealthReport>>,
}

impl HealthMonitor {
    pub fn new(config: Config, logs: Arc<LoggingBus>) -> Self {
        let initial = HealthReport::from_checks(vec![HealthCheck {
            name: "startup".to_string(),
            status: HealthStatus::Degraded,
            latency_ms: None,
            message: Some("health monitor has not run yet".to_string()),
        }]);
        Self { config, logs, latest: Arc::new(RwLock::new(initial)) }
    }

    pub async fn check_once(&self) -> HealthReport {
        let checks = vec![
            check_ping().await,
            check_iii_pubsub(&self.config).await,
        ];
        let report = HealthReport::from_checks(checks);
        *self.latest.write().await = report.clone();
        self.logs.emit(BotLogEvent::new(LogLevel::Info, "health.checked", format!("health is {:?}", report.overall))).await;
        report
    }

    pub async fn latest(&self) -> HealthReport {
        self.latest.read().await.clone()
    }

    pub async fn run_periodic(self: Arc<Self>) {
        let mut interval = tokio::time::interval(self.config.health_check_interval);
        loop {
            interval.tick().await;
            self.check_once().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AiProvider;
    use std::time::Duration;

    fn config() -> Config {
        Config {
            telegram_token_present: false,
            iii_url: "ws://127.0.0.1:49134".into(),
            ai_provider: AiProvider::Anthropic,
            ai_model: "claude-sonnet-4-6".into(),
            ai_base_url: None,
            ai_api_path: None,
            l0_history_limit: 30,
            l0_max_user_history: 15,
            l0_max_assistant_history: 15,
            l0_search_limit: 10,
            health_check_interval: Duration::from_secs(60),
            db_health_timeout: Duration::from_millis(1),
            tool_audit_log_to_l0: true,
            max_tool_failure_retries: 5,
            ai_agent_timeout: Duration::from_secs(60),
            ai_agent_max_timeout_retries: 3,
            log_level: "info".into(),
            log_to_terminal: true,
            log_to_jsonl: true,
            log_jsonl_path: "./logs/bot-events.jsonl".into(),
            log_to_database: true,
            log_to_pubsub: true,
            log_pubsub_topic: "bot.logs".into(),
        }
    }

    #[tokio::test]
    async fn configured_health_checks_are_ping_and_iii_pubsub_only() {
        let monitor = HealthMonitor::new(config(), Arc::new(LoggingBus::default()));
        let report = monitor.check_once().await;
        let names = report.checks.iter().map(|check| check.name.as_str()).collect::<Vec<_>>();

        assert_eq!(names, vec!["ping", "iii_pubsub"]);
    }
}
