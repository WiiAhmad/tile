use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub overall: HealthStatus,
    pub checked_at_ms: i64,
    pub checks: Vec<HealthCheck>,
}

impl HealthReport {
    pub fn from_checks(checks: Vec<HealthCheck>) -> Self {
        let overall = if checks.iter().any(|check| check.status == HealthStatus::Unhealthy) {
            HealthStatus::Unhealthy
        } else if checks.iter().any(|check| check.status == HealthStatus::Degraded) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        Self {
            overall,
            checked_at_ms: chrono::Utc::now().timestamp_millis(),
            checks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unhealthy_check_makes_report_unhealthy() {
        let report = HealthReport::from_checks(vec![
            HealthCheck { name: "a".into(), status: HealthStatus::Healthy, latency_ms: Some(1), message: None },
            HealthCheck { name: "b".into(), status: HealthStatus::Unhealthy, latency_ms: None, message: Some("down".into()) },
        ]);
        assert_eq!(report.overall, HealthStatus::Unhealthy);
    }

    #[test]
    fn degraded_check_makes_report_degraded_when_none_unhealthy() {
        let report = HealthReport::from_checks(vec![
            HealthCheck { name: "a".into(), status: HealthStatus::Healthy, latency_ms: Some(1), message: None },
            HealthCheck { name: "b".into(), status: HealthStatus::Degraded, latency_ms: Some(2), message: None },
        ]);
        assert_eq!(report.overall, HealthStatus::Degraded);
    }
}
