use bot::agents::service::AiService;
use bot::config::Config;
use bot::error::Result;
use bot::health::model::{HealthReport, HealthStatus};
use bot::health::monitor::HealthMonitor;
use bot::l0::fts_store::SqliteL0FtsStore;
use bot::l0::memory_repository::MemoryL0Repository;
use bot::l0::repository::L0Repository;
use bot::l0::sqlite_repository::SqliteL0Repository;
use bot::logging::events::{BotLogEvent, LogLevel};
use bot::logging::jsonl::JsonlSink;
use bot::logging::sqlite::SqliteLogSink;
use bot::logging::terminal::TerminalSink;
use bot::logging::{LogSink, LoggingBus};
use bot::telegram::handlers::BotState;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let config = Config::from_env()?;
    init_logging(&config);
    let sqlite_store = SqliteL0FtsStore::open("./data/database.db")?;
    let l0 = build_l0_repository(&sqlite_store);
    let sinks = build_log_sinks(sqlite_store.clone());
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

    if let Some(bot_token) = &config.bot_token {
        bot::telegram::dispatcher::run(teloxide::Bot::new(bot_token.clone()), state).await;
    } else {
        println!("BOT_TOKEN is not set; Telegram dispatcher not started.");
    }

    Ok(())
}

fn init_logging(config: &Config) {
    let mut builder = pretty_env_logger::formatted_timed_builder();
    builder.parse_filters(&config.log_level);
    let _ = builder.try_init();
}

fn build_l0_repository(store: &SqliteL0FtsStore) -> Arc<dyn L0Repository> {
    if std::env::var("L0_USE_MEMORY").is_ok() {
        Arc::new(MemoryL0Repository::new())
    } else {
        Arc::new(SqliteL0Repository::from_store(store.clone()))
    }
}

fn ensure_startup_health(report: &HealthReport) -> Result<()> {
    if report.overall == HealthStatus::Unhealthy && std::env::var("ALLOW_DEGRADED_START").is_err() {
        anyhow::bail!("startup health check failed; set ALLOW_DEGRADED_START=true to run anyway");
    }
    Ok(())
}

fn build_log_sinks(sqlite_store: SqliteL0FtsStore) -> Vec<Arc<dyn LogSink>> {
    let mut sinks: Vec<Arc<dyn LogSink>> = Vec::new();

    sinks.push(Arc::new(TerminalSink));

    sinks.push(Arc::new(JsonlSink::new("./logs/bot-events.jsonl")));
    sinks.push(Arc::new(SqliteLogSink::new(sqlite_store)));

    sinks
}

#[cfg(test)]
mod tests {
    use super::*;
    use bot::health::model::HealthCheck;

    #[test]
    fn log_sinks_always_include_terminal_jsonl_and_sqlite() {
        let store = SqliteL0FtsStore::in_memory().unwrap();
        let sinks = build_log_sinks(store);

        assert_eq!(sinks.len(), 3);
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
