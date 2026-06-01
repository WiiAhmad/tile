use anyhow::Context;
use bot::l0::fts_store::SqliteL0FtsStore;
use bot::l0::model::L0Record;
use iii_sdk::{register_worker, IIIError, InitOptions, RegisterFunction, StreamSetInput, TriggerRequest};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

const DEFAULT_STREAM_NAME: &str = "telegram_l0";

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let iii_url = std::env::var("III_URL").unwrap_or_else(|_| "ws://127.0.0.1:49134".to_string());
    let sqlite_path = std::env::var("L0_FTS_SQLITE_PATH").unwrap_or_else(|_| "./data/iii.db".to_string());
    let stream_name = std::env::var("L0_STREAM_NAME").unwrap_or_else(|_| DEFAULT_STREAM_NAME.to_string());

    let iii = register_worker(&iii_url, InitOptions::default());
    let store = Arc::new(SqliteL0FtsStore::open(sqlite_path)?);

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

    println!("L0 FTS worker registered l0::add, l0::list, l0::search");
    tokio::signal::ctrl_c().await?;
    iii.shutdown_async().await;
    Ok(())
}

fn to_iii_error(error: impl std::fmt::Display) -> IIIError {
    IIIError::Handler(error.to_string())
}
