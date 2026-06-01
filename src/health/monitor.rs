use crate::health::checks::check_ping;
use crate::health::model::{HealthCheck, HealthReport, HealthStatus};
use crate::logging::events::{BotLogEvent, LogLevel};
use crate::logging::LoggingBus;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct HealthMonitor {
    logs: Arc<LoggingBus>,
    latest: Arc<RwLock<HealthReport>>,
}

impl HealthMonitor {
    pub fn new(logs: Arc<LoggingBus>) -> Self {
        let initial = HealthReport::from_checks(vec![HealthCheck {
            name: "startup".to_string(),
            status: HealthStatus::Degraded,
            latency_ms: None,
            message: Some("health monitor has not run yet".to_string()),
        }]);
        Self { logs, latest: Arc::new(RwLock::new(initial)) }
    }

    pub async fn check_once(&self) -> HealthReport {
        let checks = vec![check_ping().await];
        let report = HealthReport::from_checks(checks);
        *self.latest.write().await = report.clone();
        self.logs.emit(BotLogEvent::new(LogLevel::Info, "health.checked", format!("health is {:?}", report.overall))).await;
        report
    }

    pub async fn latest(&self) -> HealthReport {
        self.latest.read().await.clone()
    }

    pub async fn run_periodic(self: Arc<Self>) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            self.check_once().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn configured_health_checks_are_ping_only() {
        let monitor = HealthMonitor::new(Arc::new(LoggingBus::default()));
        let report = monitor.check_once().await;
        let names = report.checks.iter().map(|check| check.name.as_str()).collect::<Vec<_>>();

        assert_eq!(names, vec!["ping"]);
    }
}
