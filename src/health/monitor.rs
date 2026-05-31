use crate::config::Config;
use crate::health::checks::{check_ai_provider_config, check_l0_round_trip, check_telegram_config};
use crate::health::model::{HealthCheck, HealthReport, HealthStatus};
use crate::l0::repository::L0Repository;
use crate::logging::events::{BotLogEvent, LogLevel};
use crate::logging::LoggingBus;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct HealthMonitor {
    l0: Arc<dyn L0Repository>,
    config: Config,
    logs: Arc<LoggingBus>,
    latest: Arc<RwLock<HealthReport>>,
}

impl HealthMonitor {
    pub fn new(l0: Arc<dyn L0Repository>, config: Config, logs: Arc<LoggingBus>) -> Self {
        let initial = HealthReport::from_checks(vec![HealthCheck {
            name: "startup".to_string(),
            status: HealthStatus::Degraded,
            latency_ms: None,
            message: Some("health monitor has not run yet".to_string()),
        }]);
        Self { l0, config, logs, latest: Arc::new(RwLock::new(initial)) }
    }

    pub async fn check_once(&self) -> HealthReport {
        let checks = vec![
            check_telegram_config(&self.config).await,
            check_ai_provider_config(&self.config).await,
            check_l0_round_trip(self.l0.clone(), self.config.db_health_timeout).await,
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
