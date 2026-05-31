use crate::config::{AiProvider, Config};
use crate::health::model::{HealthCheck, HealthStatus};
use crate::l0::model::L0Record;
use crate::l0::repository::L0Repository;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

pub async fn check_ai_provider_config(config: &Config) -> HealthCheck {
    let healthy = match &config.ai_provider {
        AiProvider::Anthropic => std::env::var("ANTHROPIC_API_KEY").is_ok(),
        AiProvider::OpenAi | AiProvider::OpenAiCompatible => {
            std::env::var("OPENAI_API_KEY").is_ok()
        }
    };

    HealthCheck {
        name: "ai_provider_config".to_string(),
        status: if healthy { HealthStatus::Healthy } else { HealthStatus::Degraded },
        latency_ms: None,
        message: if healthy { None } else { Some("provider API key is not set".to_string()) },
    }
}

pub async fn check_telegram_config(config: &Config) -> HealthCheck {
    HealthCheck {
        name: "telegram_config".to_string(),
        status: if config.telegram_token_present { HealthStatus::Healthy } else { HealthStatus::Unhealthy },
        latency_ms: None,
        message: if config.telegram_token_present { None } else { Some("TELOXIDE_TOKEN is not set".to_string()) },
    }
}

pub async fn check_l0_round_trip(repo: Arc<dyn L0Repository>, timeout: std::time::Duration) -> HealthCheck {
    let started = Instant::now();
    let id = Uuid::new_v4().to_string();
    let record = L0Record::new_user(
        id,
        "health:l0".to_string(),
        0,
        None,
        None,
        "health-check".to_string(),
        chrono::Utc::now().timestamp_millis(),
    );

    let result = tokio::time::timeout(timeout, async {
        repo.add(record).await?;
        repo.list("health:l0", 1).await?;
        Ok::<(), anyhow::Error>(())
    })
    .await;

    match result {
        Ok(Ok(())) => HealthCheck {
            name: "l0_round_trip".to_string(),
            status: HealthStatus::Healthy,
            latency_ms: Some(started.elapsed().as_millis() as u64),
            message: None,
        },
        Ok(Err(error)) => HealthCheck {
            name: "l0_round_trip".to_string(),
            status: HealthStatus::Unhealthy,
            latency_ms: Some(started.elapsed().as_millis() as u64),
            message: Some(error.to_string()),
        },
        Err(_) => HealthCheck {
            name: "l0_round_trip".to_string(),
            status: HealthStatus::Unhealthy,
            latency_ms: Some(started.elapsed().as_millis() as u64),
            message: Some("timeout".to_string()),
        },
    }
}
