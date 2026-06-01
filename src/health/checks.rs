use crate::config::Config;
use crate::health::model::{HealthCheck, HealthStatus};
use crate::logging::events::{BotLogEvent, LogLevel};
use crate::logging::pubsub::PubsubSink;
use crate::logging::LogSink;
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

pub async fn check_iii_pubsub(config: &Config) -> HealthCheck {
    let started = Instant::now();
    let timeout = config.db_health_timeout;
    let sink = PubsubSink::with_worker(&config.iii_url, config.log_pubsub_topic.clone())
        .with_timeout_ms(timeout.as_millis().min(u128::from(u64::MAX)) as u64);
    let event = BotLogEvent::new(LogLevel::Info, "health.iii_pubsub", "health pubsub ping");

    let result = tokio::time::timeout(timeout, sink.emit(&event)).await;
    match result {
        Ok(Ok(())) => HealthCheck {
            name: "iii_pubsub".to_string(),
            status: HealthStatus::Healthy,
            latency_ms: Some(started.elapsed().as_millis() as u64),
            message: None,
        },
        Ok(Err(error)) => HealthCheck {
            name: "iii_pubsub".to_string(),
            status: HealthStatus::Unhealthy,
            latency_ms: Some(started.elapsed().as_millis() as u64),
            message: Some(error.to_string()),
        },
        Err(_) => HealthCheck {
            name: "iii_pubsub".to_string(),
            status: HealthStatus::Unhealthy,
            latency_ms: Some(started.elapsed().as_millis() as u64),
            message: Some("timeout".to_string()),
        },
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
