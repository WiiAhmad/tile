use crate::health::model::{HealthCheck, HealthStatus};
use std::time::Instant;

pub async fn check_ping() -> HealthCheck {
    let started = Instant::now();
    HealthCheck {
        name: "ping".to_string(),
        status: HealthStatus::Healthy,
        latency_ms: Some(started.elapsed().as_millis() as u64),
        message: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ping_check_is_healthy() {
        let check = check_ping().await;

        assert_eq!(check.name, "ping");
        assert_eq!(check.status, HealthStatus::Healthy);
        assert!(check.latency_ms.is_some());
        assert_eq!(check.message, None);
    }
}
