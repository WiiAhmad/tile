use crate::error::Result;
use crate::l0::model::L0Record;
use crate::l0::repository::L0Repository;
use crate::l0::search::search_records;
use anyhow::Context;
use async_trait::async_trait;
use iii_sdk::{register_worker, InitOptions, StreamListInput, StreamSetInput, TriggerRequest, III};

const DEFAULT_STREAM_NAME: &str = "telegram_l0";

#[derive(Clone)]
pub struct IiiL0Repository {
    iii: III,
    stream_name: String,
    timeout_ms: Option<u64>,
    use_worker_functions: bool,
}

impl IiiL0Repository {
    pub fn new(iii_url: impl AsRef<str>) -> Self {
        Self::with_stream(iii_url, DEFAULT_STREAM_NAME)
    }

    pub fn with_stream(iii_url: impl AsRef<str>, stream_name: impl Into<String>) -> Self {
        let iii = register_worker(iii_url.as_ref(), InitOptions::default());
        Self::from_client(iii, stream_name)
    }

    pub fn from_client(iii: III, stream_name: impl Into<String>) -> Self {
        Self {
            iii,
            stream_name: stream_name.into(),
            timeout_ms: None,
            use_worker_functions: false,
        }
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_worker_functions(mut self, enabled: bool) -> Self {
        self.use_worker_functions = enabled;
        self
    }

    fn trigger_request(&self, function_id: &str, payload: serde_json::Value) -> TriggerRequest {
        TriggerRequest {
            function_id: function_id.to_string(),
            payload,
            action: None,
            timeout_ms: self.timeout_ms,
        }
    }

    async fn stream_add(&self, record: L0Record) -> Result<()> {
        let payload = serde_json::to_value(StreamSetInput {
            stream_name: self.stream_name.clone(),
            group_id: record.conversation_id.clone(),
            item_id: record.id.clone(),
            data: serde_json::to_value(record)?,
        })?;

        self.iii
            .trigger(self.trigger_request("stream::set", payload))
            .await
            .context("iii stream::set failed")?;
        Ok(())
    }

    async fn stream_list(&self, conversation_id: &str, limit: usize) -> Result<Vec<L0Record>> {
        let payload = serde_json::to_value(StreamListInput {
            stream_name: self.stream_name.clone(),
            group_id: conversation_id.to_string(),
        })?;

        let value = self
            .iii
            .trigger(self.trigger_request("stream::list", payload))
            .await
            .context("iii stream::list failed")?;
        let mut records = records_from_stream_list_value(value)?;
        records.retain(|record| record.conversation_id == conversation_id);
        records.sort_by_key(|record| record.created_at_ms);
        let start = records.len().saturating_sub(limit);
        Ok(records[start..].to_vec())
    }

    async fn stream_search(&self, conversation_id: &str, query: &str, limit: usize) -> Result<Vec<L0Record>> {
        let listed = self.stream_list(conversation_id, usize::MAX).await?;
        Ok(search_records(&listed, query, limit))
    }

    async fn worker_add(&self, record: &L0Record) -> Result<()> {
        self.iii
            .trigger(self.trigger_request("l0::add", serde_json::json!({ "record": record })))
            .await
            .context("iii l0::add failed")?;
        Ok(())
    }

    async fn worker_list(&self, conversation_id: &str, limit: usize) -> Result<Vec<L0Record>> {
        let value = self
            .iii
            .trigger(self.trigger_request(
                "l0::list",
                serde_json::json!({ "conversation_id": conversation_id, "limit": limit }),
            ))
            .await
            .context("iii l0::list failed")?;
        records_from_l0_worker_value(value, "records")
    }

    async fn worker_search(&self, conversation_id: &str, query: &str, limit: usize) -> Result<Vec<L0Record>> {
        let value = self
            .iii
            .trigger(self.trigger_request(
                "l0::search",
                serde_json::json!({ "conversation_id": conversation_id, "query": query, "limit": limit }),
            ))
            .await
            .context("iii l0::search failed")?;
        records_from_l0_worker_value(value, "results")
    }
}

#[async_trait]
impl L0Repository for IiiL0Repository {
    async fn add(&self, record: L0Record) -> Result<()> {
        if self.use_worker_functions && self.worker_add(&record).await.is_ok() {
            return Ok(());
        }
        self.stream_add(record).await
    }

    async fn list(&self, conversation_id: &str, limit: usize) -> Result<Vec<L0Record>> {
        if self.use_worker_functions {
            if let Ok(records) = self.worker_list(conversation_id, limit).await {
                return Ok(records);
            }
        }
        self.stream_list(conversation_id, limit).await
    }

    async fn search(&self, conversation_id: &str, query: &str, limit: usize) -> Result<Vec<L0Record>> {
        if self.use_worker_functions {
            if let Ok(records) = self.worker_search(conversation_id, query, limit).await {
                return Ok(records);
            }
        }
        self.stream_search(conversation_id, query, limit).await
    }
}

fn records_from_l0_worker_value(mut value: serde_json::Value, field: &str) -> Result<Vec<L0Record>> {
    if let Some(object) = value.as_object_mut() {
        if let Some(records) = object.remove(field).or_else(|| object.remove("records")) {
            return records_from_stream_list_value(records);
        }
    }
    records_from_stream_list_value(value)
}

fn records_from_stream_list_value(value: serde_json::Value) -> Result<Vec<L0Record>> {
    match value {
        serde_json::Value::Array(items) => parse_record_items(items),
        serde_json::Value::Object(mut object) => {
            if let Some(items) = object.remove("items").or_else(|| object.remove("records")) {
                records_from_stream_list_value(items)
            } else if let Some(data) = object.remove("data") {
                records_from_stream_list_value(data)
            } else {
                anyhow::bail!("unexpected iii stream::list object shape")
            }
        }
        serde_json::Value::Null => Ok(Vec::new()),
        other => anyhow::bail!("unexpected iii stream::list response: {other}"),
    }
}

fn parse_record_items(items: Vec<serde_json::Value>) -> Result<Vec<L0Record>> {
    let mut records = Vec::with_capacity(items.len());
    for item in items {
        match item {
            serde_json::Value::Object(mut object) => {
                let value = object
                    .remove("data")
                    .or_else(|| object.remove("value"))
                    .unwrap_or(serde_json::Value::Object(object));
                records.push(serde_json::from_value(value).context("invalid L0 record in iii stream item")?);
            }
            other => records.push(serde_json::from_value(other).context("invalid L0 record in iii stream item")?),
        }
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l0::model::L0Record;

    fn user(id: &str, conversation_id: &str, created_at_ms: i64) -> L0Record {
        L0Record::new_user(
            id.to_string(),
            conversation_id.to_string(),
            1,
            None,
            None,
            format!("message {id}"),
            created_at_ms,
        )
    }

    #[test]
    fn parses_raw_record_arrays() {
        let records = records_from_stream_list_value(serde_json::json!([
            user("1", "telegram:1", 1),
            user("2", "telegram:1", 2)
        ]))
        .unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id, "1");
    }

    #[test]
    fn parses_wrapped_stream_items() {
        let records = records_from_stream_list_value(serde_json::json!({
            "items": [
                { "item_id": "1", "data": user("1", "telegram:1", 1) }
            ]
        }))
        .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].conversation_id, "telegram:1");
    }
}
