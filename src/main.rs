use bot::agents::service::AiService;
use bot::config::Config;
use bot::error::Result;
use bot::health::model::{HealthReport, HealthStatus};
use bot::health::monitor::HealthMonitor;
use bot::l0::iii_repository::IiiL0Repository;
use bot::l0::memory_repository::MemoryL0Repository;
use bot::l0::repository::L0Repository;
use bot::logging::events::{BotLogEvent, LogLevel};
use bot::logging::jsonl::JsonlSink;
use bot::logging::pubsub::PubsubSink;
use bot::logging::terminal::TerminalSink;
use bot::logging::{LogSink, LoggingBus};
use bot::telegram::handlers::BotState;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let config = Config::from_env()?;
    init_logging(&config);
    let l0 = build_l0_repository(&config);
    let sinks = build_log_sinks(&config);
    let logs = Arc::new(LoggingBus::new(sinks));

    let health = Arc::new(HealthMonitor::new(config.clone(), logs.clone()));
    let startup_report = health.check_once().await;
    ensure_startup_health(&startup_report)?;
    tokio::spawn(health.clone().run_periodic());

    let ai = Arc::new(AiService::new(config.clone(), l0.clone(), logs.clone()));
    let state = Arc::new(BotState::new(
        config.clone(),
        l0,
        logs.clone(),
        health,
        ai,
    ));

    logs.emit(BotLogEvent::new(
        LogLevel::Info,
        "runtime.initialized",
        "Telegram AI L0 bot initialized",
    ))
    .await;
    println!("Telegram AI L0 bot initialized");

    if config.telegram_token_present {
        bot::telegram::dispatcher::run(teloxide::Bot::from_env(), state).await;
    } else {
        println!("TELOXIDE_TOKEN is not set; Telegram dispatcher not started.");
    }

    Ok(())
}

fn init_logging(config: &Config) {
    let mut builder = pretty_env_logger::formatted_timed_builder();
    builder.parse_filters(&config.log_level);
    let _ = builder.try_init();
}

fn build_l0_repository(config: &Config) -> Arc<dyn L0Repository> {
    if std::env::var("L0_USE_MEMORY").is_ok() {
        Arc::new(MemoryL0Repository::new())
    } else {
        Arc::new(
            IiiL0Repository::new(&config.iii_url)
                .with_timeout_ms(config.db_health_timeout.as_millis().min(u128::from(u64::MAX)) as u64)
                .with_worker_functions(config.l0_use_worker_functions),
        )
    }
}

fn ensure_startup_health(report: &HealthReport) -> Result<()> {
    if report.overall == HealthStatus::Unhealthy && std::env::var("ALLOW_DEGRADED_START").is_err() {
        anyhow::bail!("startup health check failed; set ALLOW_DEGRADED_START=true to run anyway");
    }
    Ok(())
}

fn build_log_sinks(config: &Config) -> Vec<Arc<dyn LogSink>> {
    let mut sinks: Vec<Arc<dyn LogSink>> = Vec::new();

    if config.log_to_terminal {
        sinks.push(Arc::new(TerminalSink));
    }

    if config.log_to_jsonl {
        sinks.push(Arc::new(JsonlSink::new(config.log_jsonl_path.clone())));
    }

    if config.log_to_database {
        // User, assistant, tool, and health records are already persisted through the L0 repository.
        // A dedicated observability-to-L0 sink can be added later if log events need separate records.
    }

    if config.log_to_pubsub {
        let timeout_ms = config.db_health_timeout.as_millis().min(u128::from(u64::MAX)) as u64;
        sinks.push(Arc::new(
            PubsubSink::with_worker(&config.iii_url, config.log_pubsub_topic.clone())
                .with_timeout_ms(timeout_ms),
        ));
    }

    sinks
}

#[cfg(test)]
mod tests {
    use super::*;
    use bot::health::model::HealthCheck;

    #[test]
    fn log_sinks_do_not_include_websocket_sink() {
        let config = Config::from_env().unwrap_or_else(|_| panic!("env config should parse for this test"));
        let sinks = build_log_sinks(&config);
        assert!(sinks.len() <= 3);
    }

    #[test]
    fn startup_health_rejects_unhealthy_report() {
        let report = HealthReport::from_checks(vec![HealthCheck {
            name: "l0".to_string(),
            status: HealthStatus::Unhealthy,
            latency_ms: None,
            message: Some("down".to_string()),
        }]);

        let error = ensure_startup_health(&report).unwrap_err().to_string();

        assert!(error.contains("startup health check failed"));
    }
}
