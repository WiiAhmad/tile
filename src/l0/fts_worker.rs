use crate::l0::fts_store::SqliteL0FtsStore;
use crate::l0::model::L0Record;
use anyhow::Context;
use iii_sdk::{register_worker, IIIError, InitOptions, RegisterFunction, StreamSetInput, TriggerRequest, III};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

pub const DEFAULT_STREAM_NAME: &str = "telegram_l0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct L0FtsWorkerConfig {
    pub iii_url: String,
    pub sqlite_path: String,
    pub stream_name: String,
}

impl L0FtsWorkerConfig {
    pub fn from_env() -> Self {
        Self {
            iii_url: std::env::var("III_URL").unwrap_or_else(|_| "ws://127.0.0.1:49134".to_string()),
            sqlite_path: std::env::var("L0_FTS_SQLITE_PATH").unwrap_or_else(|_| "./data/iii.db".to_string()),
            stream_name: std::env::var("L0_STREAM_NAME").unwrap_or_else(|_| DEFAULT_STREAM_NAME.to_string()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct AddRequest {
    record: L0Record,
}

#[derive(Debug, Deserialize)]
struct ListRequest {
    conversation_id: String,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct SearchRequest {
    conversation_id: String,
    query: String,
    limit: Option<usize>,
}

pub async fn run_from_env() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    run(L0FtsWorkerConfig::from_env()).await
}

pub async fn run(config: L0FtsWorkerConfig) -> anyhow::Result<()> {
    let iii = register_worker(&config.iii_url, InitOptions::default());
    let store = Arc::new(SqliteL0FtsStore::open(config.sqlite_path)?);
    register_functions(&iii, store, config.stream_name);

    println!("L0 FTS worker registered l0::add, l0::list, l0::search");
    tokio::signal::ctrl_c().await?;
    iii.shutdown_async().await;
    Ok(())
}

fn register_functions(iii: &III, store: Arc<SqliteL0FtsStore>, stream_name: String) {
    iii.register_function(
        "l0::add",
        RegisterFunction::new_async({
            let iii = iii.clone();
            let store = store.clone();
            let stream_name = stream_name.clone();
            move |payload: Value| {
                let iii = iii.clone();
                let store = store.clone();
                let stream_name = stream_name.clone();
                async move {
                    let request: AddRequest = serde_json::from_value(payload).map_err(to_iii_error)?;
                    let record = request.record;

                    let stream_payload = serde_json::to_value(StreamSetInput {
                        stream_name,
                        group_id: record.conversation_id.clone(),
                        item_id: record.id.clone(),
                        data: serde_json::to_value(&record).map_err(to_iii_error)?,
                    })
                    .map_err(to_iii_error)?;
                    iii.trigger(TriggerRequest {
                        function_id: "stream::set".to_string(),
                        payload: stream_payload,
                        action: None,
                        timeout_ms: None,
                    })
                    .await
                    .map_err(to_iii_error)
                    .context("l0::add stream::set failed")
                    .map_err(to_iii_error)?;

                    store
                        .add(&record)
                        .context("l0::add sqlite index failed")
                        .map_err(to_iii_error)?;
                    Ok(json!({ "ok": true }))
                }
            }
        }),
    );

    iii.register_function(
        "l0::list",
        RegisterFunction::new_async({
            let store = store.clone();
            move |payload: Value| {
                let store = store.clone();
                async move {
                    let request: ListRequest = serde_json::from_value(payload).map_err(to_iii_error)?;
                    let records = store
                        .list(&request.conversation_id, request.limit.unwrap_or(10))
                        .context("l0::list sqlite list failed")
                        .map_err(to_iii_error)?;
                    Ok(json!({ "ok": true, "records": records }))
                }
            }
        }),
    );

    iii.register_function(
        "l0::search",
        RegisterFunction::new_async({
            let store = store.clone();
            move |payload: Value| {
                let store = store.clone();
                async move {
                    let request: SearchRequest = serde_json::from_value(payload).map_err(to_iii_error)?;
                    let records = store
                        .search(&request.conversation_id, &request.query, request.limit.unwrap_or(10))
                        .context("l0::search hybrid search failed")
                        .map_err(to_iii_error)?;
                    Ok(json!({ "ok": true, "results": records }))
                }
            }
        }),
    );
}

fn to_iii_error(error: impl std::fmt::Display) -> IIIError {
    IIIError::Handler(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_config_uses_expected_defaults() {
        let config = L0FtsWorkerConfig::from_env();

        assert_eq!(config.iii_url, "ws://127.0.0.1:49134");
        assert_eq!(config.sqlite_path, "./data/iii.db");
        assert_eq!(config.stream_name, DEFAULT_STREAM_NAME);
    }
}
