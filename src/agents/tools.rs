use crate::agents::tool_hooks::{post_tool_failure, post_tool_use, pre_tool_use};
use crate::l0::repository::L0Repository;
use crate::logging::LoggingBus;
use crate::types::TelegramMeta;
use aisdk::core::tools::{Tool, ToolExecute};
use schemars::{schema_for, JsonSchema, Schema};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Reference only: L0 writes are now automatic from Telegram/user/assistant flow, so the AI
// should not be offered a write-memory tool. Keep this here to make re-enabling explicit.
//
// use crate::error::Result;
// use crate::l0::model::{L0Record, L0Role, L0Source};
// use uuid::Uuid;
//
// #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
// pub struct L0AddInput {
//     pub content: String,
//     pub role: Option<L0Role>,
//     pub tags: Option<Vec<String>>,
// }
//
// pub async fn run_l0_add_tool(
//     input: L0AddInput,
//     runtime: TelegramMeta,
//     l0: Arc<dyn L0Repository>,
//     logs: Arc<LoggingBus>,
// ) -> serde_json::Value {
//     let raw_args = serde_json::to_value(&input).unwrap_or_else(|_| serde_json::json!({}));
//     let ctx = match pre_tool_use("l0_add", raw_args, runtime, l0.clone(), logs.clone()).await {
//         Ok(ctx) => ctx,
//         Err(error) => return pre_tool_error(error),
//     };
//
//     let result: Result<serde_json::Value> = async {
//         let id = Uuid::new_v4().to_string();
//         l0.add(L0Record {
//             id: id.clone(),
//             conversation_id: ctx.conversation_id.clone(),
//             telegram_chat_id: ctx.telegram_chat_id,
//             telegram_user_id: ctx.telegram_user_id,
//             telegram_message_id: ctx.telegram_message_id,
//             role: input.role.unwrap_or(L0Role::Tool),
//             content: input.content,
//             source: L0Source::Manual,
//             provider: None,
//             model: None,
//             tool_name: Some(ctx.tool_name.clone()),
//             tool_call_id: Some(ctx.trace_id.clone()),
//             raw_json: Some(serde_json::json!({ "tags": input.tags.unwrap_or_default() })),
//             created_at_ms: chrono::Utc::now().timestamp_millis(),
//         }).await?;
//         Ok(serde_json::json!({ "ok": true, "id": id }))
//     }.await;
//
//     match result {
//         Ok(value) => {
//             let _ = post_tool_use(&ctx, &value, l0, logs).await;
//             value
//         }
//         Err(error) => post_tool_failure(&ctx, &error, l0, logs).await,
//     }
// }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct L0SearchInput {
    pub query: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct L0ListInput {
    pub limit: Option<u32>,
}

pub async fn run_l0_search_tool(
    input: L0SearchInput,
    runtime: TelegramMeta,
    l0: Arc<dyn L0Repository>,
    logs: Arc<LoggingBus>,
) -> serde_json::Value {
    let raw_args = serde_json::to_value(&input).unwrap_or_else(|_| serde_json::json!({}));
    let ctx = match pre_tool_use("l0_search", raw_args, runtime, l0.clone(), logs.clone()).await {
        Ok(ctx) => ctx,
        Err(error) => return pre_tool_error(error),
    };

    let limit = input.limit.unwrap_or(10).min(50) as usize;
    match l0.search(&ctx.conversation_id, &input.query, limit).await {
        Ok(records) => {
            let value = serde_json::json!({ "ok": true, "results": records });
            let _ = post_tool_use(&ctx, &value, l0, logs).await;
            value
        }
        Err(error) => post_tool_failure(&ctx, &error, l0, logs).await,
    }
}

pub async fn run_l0_list_tool(
    input: L0ListInput,
    runtime: TelegramMeta,
    l0: Arc<dyn L0Repository>,
    logs: Arc<LoggingBus>,
) -> serde_json::Value {
    let raw_args = serde_json::to_value(&input).unwrap_or_else(|_| serde_json::json!({}));
    let ctx = match pre_tool_use("l0_list", raw_args, runtime, l0.clone(), logs.clone()).await {
        Ok(ctx) => ctx,
        Err(error) => return pre_tool_error(error),
    };

    let limit = input.limit.unwrap_or(10).min(50) as usize;
    match l0.list(&ctx.conversation_id, limit).await {
        Ok(records) => {
            let value = serde_json::json!({ "ok": true, "records": records });
            let _ = post_tool_use(&ctx, &value, l0, logs).await;
            value
        }
        Err(error) => post_tool_failure(&ctx, &error, l0, logs).await,
    }
}

pub fn l0_tools(runtime: TelegramMeta, l0: Arc<dyn L0Repository>, logs: Arc<LoggingBus>) -> Vec<Tool> {
    vec![
        // Reference only: disabled because L0 writes are automatic.
        // Tool {
        //     name: "l0_add".to_string(),
        //     description: "Store a raw L0 memory/event scoped to the current Telegram conversation.".to_string(),
        //     input_schema: openai_tool_schema::<L0AddInput>(),
        //     execute: ToolExecute::new(Box::new({
        //         let runtime = runtime.clone();
        //         let l0 = l0.clone();
        //         let logs = logs.clone();
        //         move |params| run_tool_sync(run_l0_add_tool(
        //             serde_json::from_value(params).map_err(|error| error.to_string())?,
        //             runtime.clone(),
        //             l0.clone(),
        //             logs.clone(),
        //         ))
        //     })),
        // },
        Tool {
            name: "l0_search".to_string(),
            description: "Search raw L0 records in the current Telegram conversation.".to_string(),
            input_schema: openai_tool_schema::<L0SearchInput>(),
            execute: ToolExecute::new(Box::new({
                let runtime = runtime.clone();
                let l0 = l0.clone();
                let logs = logs.clone();
                move |params| run_tool_sync(run_l0_search_tool(
                    serde_json::from_value(params).map_err(|error| error.to_string())?,
                    runtime.clone(),
                    l0.clone(),
                    logs.clone(),
                ))
            })),
        },
        Tool {
            name: "l0_list".to_string(),
            description: "List recent raw L0 records in the current Telegram conversation.".to_string(),
            input_schema: openai_tool_schema::<L0ListInput>(),
            execute: ToolExecute::new(Box::new(move |params| run_tool_sync(run_l0_list_tool(
                serde_json::from_value(params).map_err(|error| error.to_string())?,
                runtime.clone(),
                l0.clone(),
                logs.clone(),
            )))),
        },
    ]
}

fn openai_tool_schema<T: JsonSchema>() -> Schema {
    let mut schema = serde_json::to_value(schema_for!(T)).expect("schema must serialize");
    if let Some(object) = schema.as_object_mut() {
        if let Some(properties) = object.get("properties").and_then(|value| value.as_object()) {
            let required = properties
                .keys()
                .cloned()
                .map(serde_json::Value::String)
                .collect::<Vec<_>>();
            object.insert("required".to_string(), serde_json::Value::Array(required));
        }
    }
    serde_json::from_value(schema).expect("schema must deserialize")
}

fn run_tool_sync<F>(future: F) -> std::result::Result<String, String>
where
    F: std::future::Future<Output = serde_json::Value>,
{
    let value = tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future));
    serde_json::to_string(&value).map_err(|error| error.to_string())
}

fn pre_tool_error(error: anyhow::Error) -> serde_json::Value {
    serde_json::json!({
        "ok": false,
        "error": {
            "code": "pre_tool_use_failed",
            "message": error.to_string()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l0::memory_repository::MemoryL0Repository;
    use crate::l0::model::L0Record;

    #[test]
    fn registered_l0_tools_expose_only_read_tools() {
        let tools = l0_tools(
            TelegramMeta::from_chat(1, Some(2), Some(3)),
            Arc::new(MemoryL0Repository::new()),
            Arc::new(LoggingBus::default()),
        );
        let tool_names = tools.iter().map(|tool| tool.name.as_str()).collect::<Vec<_>>();

        assert_eq!(tool_names, vec!["l0_search", "l0_list"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn registered_l0_tools_can_list_current_conversation() {
        let repo = Arc::new(MemoryL0Repository::new());
        repo.add(L0Record::new_user(
            "id-1".to_string(),
            "telegram:1".to_string(),
            1,
            Some(2),
            Some(3),
            "hello memory".to_string(),
            1,
        ))
        .await
        .unwrap();
        let tools = l0_tools(
            TelegramMeta::from_chat(1, Some(2), Some(3)),
            repo,
            Arc::new(LoggingBus::default()),
        );
        let list_tool = tools.iter().find(|tool| tool.name == "l0_list").unwrap();

        let output = list_tool
            .execute
            .call(serde_json::json!({ "limit": 10 }))
            .unwrap();

        assert!(output.contains("hello memory"));
    }
}
