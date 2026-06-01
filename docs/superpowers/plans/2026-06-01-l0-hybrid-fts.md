# L0 Hybrid FTS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add hybrid L0 search that combines exact substring matching with SQLite FTS-ranked keyword search through iii worker functions, and make it transparently power Telegram `/l0search` and the AI-visible `l0_search` tool.

**Architecture:** Keep `L0Repository` as the bot-facing interface. Add a separate iii worker binary that registers `l0::add`, `l0::list`, and `l0::search`; it writes raw L0 records to iii stream and indexes `L0Record.content` in SQLite FTS. Update `IiiL0Repository` to optionally call those custom functions and fall back to the current `stream::set`/`stream::list` + substring search path when worker functions are disabled or fail.

**Tech Stack:** Rust 2024, `iii-sdk`, `rusqlite` with bundled SQLite/FTS5, `serde`, `tokio`, existing `aisdk` tool integration, existing Telegram command handlers.

---

## Design/spec reference

Spec file: `docs/superpowers/specs/2026-06-01-l0-hybrid-fts-design.md`

Project instruction conflict note: the generic planning skill recommends frequent commits, but this repository's `CLAUDE.md` says not to commit unless explicitly asked. Therefore this plan uses **checkpoint steps** (`git status`, targeted tests) instead of `git commit` steps. Only commit if the user explicitly asks.

---

## File structure

Create:

- `src/l0/fts_store.rs` — SQLite schema, indexing, list, hybrid search, query sanitization, unit tests.
- `src/bin/l0_fts_worker.rs` — standalone iii worker registering `l0::add`, `l0::list`, and `l0::search`.

Modify:

- `Cargo.toml` — add `rusqlite` and test helper dependency if needed.
- `src/l0/mod.rs` — export `fts_store`.
- `src/config.rs` — add `L0_USE_WORKER_FUNCTIONS` and `L0_FTS_SQLITE_PATH` config.
- `src/main.rs` — pass worker-function preference into `IiiL0Repository`.
- `src/l0/iii_repository.rs` — add optional custom iii L0 function calls with stream fallback.
- `src/agents/tools.rs` — keep same schema, update description, add test proving `l0_search` uses repository search.
- `src/agents/prompts.rs` — update wording so the model understands `l0_search` is hybrid keyword/phrase search over content.
- `.env.example` — document new env vars.
- `README.md` or `DEV.md` — document how to run the FTS worker and enable worker search.
- Test config builders in `src/agents/provider.rs`, `src/agents/service.rs`, and `src/health/monitor.rs` if `Config` struct literals fail after adding fields.

---

### Task 1: Add dependencies and configuration flags

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/config.rs`
- Modify: `.env.example`
- Possibly modify test config builders in:
  - `src/agents/provider.rs`
  - `src/agents/service.rs`
  - `src/health/monitor.rs`

- [ ] **Step 1: Add dependencies**

Modify `Cargo.toml` dependencies to include `rusqlite`:

```toml
[dependencies]
aisdk = { version = "0.5.2", features = ["openai", "anthropic", "openaicompatible"] }
anyhow = "1.0"
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
dotenv = "0.15.0"
iii-sdk = "0.16.1"
log = "0.4.30"
pretty_env_logger = "0.5"
rusqlite = { version = "0.37", features = ["bundled"] }
schemars = "1.2.1"
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0"
teloxide = { version = "0.17.0", features = ["macros"] }
tokio = { version = "1.52.3", features = ["full"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
```

- [ ] **Step 2: Extend `Config`**

In `src/config.rs`, add fields to the `Config` struct next to the existing L0 fields:

```rust
pub l0_history_limit: usize,
pub l0_max_user_history: usize,
pub l0_max_assistant_history: usize,
pub l0_search_limit: usize,
pub l0_use_worker_functions: bool,
pub l0_fts_sqlite_path: String,
```

In `Config::from_env`, add parsing next to the existing L0 env vars:

```rust
l0_history_limit: env_usize("L0_HISTORY_LIMIT", 30)?,
l0_max_user_history: env_usize("L0_MAX_USER_HISTORY", 15)?,
l0_max_assistant_history: env_usize("L0_MAX_ASSISTANT_HISTORY", 15)?,
l0_search_limit: env_usize("L0_SEARCH_LIMIT", 10)?,
l0_use_worker_functions: env_bool("L0_USE_WORKER_FUNCTIONS", false)?,
l0_fts_sqlite_path: env_string("L0_FTS_SQLITE_PATH", "./data/iii.db"),
```

- [ ] **Step 3: Update `.env.example`**

Add the new env vars below `L0_SEARCH_LIMIT=10`:

```dotenv
L0_SEARCH_LIMIT=10
# Enable custom iii L0 worker functions: l0::add, l0::list, l0::search.
L0_USE_WORKER_FUNCTIONS=false
# SQLite database path used by the L0 FTS worker.
L0_FTS_SQLITE_PATH=./data/iii.db
```

- [ ] **Step 4: Run config-focused check**

Run:

```bash
cargo check
```

Expected: either PASS, or compile errors only where `Config` struct literals in tests/builders need the two new fields.

- [ ] **Step 5: Fix any `Config` struct literal errors**

For each failing test helper that creates `Config { ... }`, add:

```rust
l0_use_worker_functions: false,
l0_fts_sqlite_path: "./data/iii.db".to_string(),
```

Do not change runtime defaults elsewhere.

- [ ] **Step 6: Re-run check**

Run:

```bash
cargo check
```

Expected: PASS.

- [ ] **Step 7: Checkpoint**

Run:

```bash
git status --short
```

Expected: shows modified `Cargo.toml`, `Cargo.lock`, `src/config.rs`, `.env.example`, and any test helper files touched. Do not commit unless the user explicitly asks.

---

### Task 2: Implement SQLite FTS store with hybrid search

**Files:**
- Create: `src/l0/fts_store.rs`
- Modify: `src/l0/mod.rs`

- [ ] **Step 1: Export the new module**

Modify `src/l0/mod.rs` to include:

```rust
pub mod fts_store;
pub mod iii_repository;
pub mod memory_repository;
pub mod model;
pub mod repository;
pub mod search;
```

- [ ] **Step 2: Create failing tests first**

Create `src/l0/fts_store.rs` with the tests and minimal imports below. These tests define the required behavior before implementation:

```rust
use crate::error::Result;
use crate::l0::model::L0Record;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SqliteL0FtsStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteL0FtsStore {
    pub fn in_memory() -> Result<Self> {
        todo!("implemented after failing tests")
    }

    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let _ = path;
        todo!("implemented after failing tests")
    }

    pub fn add(&self, record: &L0Record) -> Result<()> {
        let _ = record;
        todo!("implemented after failing tests")
    }

    pub fn list(&self, conversation_id: &str, limit: usize) -> Result<Vec<L0Record>> {
        let _ = (conversation_id, limit);
        todo!("implemented after failing tests")
    }

    pub fn search(&self, conversation_id: &str, query: &str, limit: usize) -> Result<Vec<L0Record>> {
        let _ = (conversation_id, query, limit);
        todo!("implemented after failing tests")
    }
}

fn sanitized_fts_query(query: &str) -> Option<String> {
    let _ = query;
    todo!("implemented after failing tests")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l0::model::L0Record;

    fn user(id: &str, conversation_id: &str, content: &str, created_at_ms: i64) -> L0Record {
        L0Record::new_user(
            id.to_string(),
            conversation_id.to_string(),
            1,
            Some(2),
            Some(3),
            content.to_string(),
            created_at_ms,
        )
    }

    #[test]
    fn lists_records_by_conversation_and_limit() {
        let store = SqliteL0FtsStore::in_memory().unwrap();
        store.add(&user("1", "telegram:1", "one", 1)).unwrap();
        store.add(&user("2", "telegram:2", "two", 2)).unwrap();
        store.add(&user("3", "telegram:1", "three", 3)).unwrap();

        let records = store.list("telegram:1", 1).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "3");
    }

    #[test]
    fn hybrid_search_keeps_exact_substring_behavior() {
        let store = SqliteL0FtsStore::in_memory().unwrap();
        store.add(&user("1", "telegram:1", "my favorite editor is Helix", 1)).unwrap();
        store.add(&user("2", "telegram:1", "editor notes without phrase", 2)).unwrap();

        let records = store.search("telegram:1", "favorite editor", 10).unwrap();

        assert_eq!(records.first().map(|record| record.id.as_str()), Some("1"));
    }

    #[test]
    fn hybrid_search_finds_words_out_of_order() {
        let store = SqliteL0FtsStore::in_memory().unwrap();
        store.add(&user("1", "telegram:1", "I configured Helix today", 1)).unwrap();
        store.add(&user("2", "telegram:1", "unrelated", 2)).unwrap();

        let records = store.search("telegram:1", "helix config", 10).unwrap();

        assert_eq!(records.iter().map(|record| record.id.as_str()).collect::<Vec<_>>(), vec!["1"]);
    }

    #[test]
    fn search_is_scoped_to_conversation() {
        let store = SqliteL0FtsStore::in_memory().unwrap();
        store.add(&user("1", "telegram:1", "helix private", 1)).unwrap();
        store.add(&user("2", "telegram:2", "helix other chat", 2)).unwrap();

        let records = store.search("telegram:1", "helix", 10).unwrap();

        assert_eq!(records.iter().map(|record| record.id.as_str()).collect::<Vec<_>>(), vec!["1"]);
    }

    #[test]
    fn exact_hits_are_returned_before_loose_fts_hits() {
        let store = SqliteL0FtsStore::in_memory().unwrap();
        store.add(&user("1", "telegram:1", "editor favorite separated words", 1)).unwrap();
        store.add(&user("2", "telegram:1", "my favorite editor is Helix", 2)).unwrap();

        let records = store.search("telegram:1", "favorite editor", 10).unwrap();

        assert_eq!(records.iter().map(|record| record.id.as_str()).collect::<Vec<_>>(), vec!["2", "1"]);
    }

    #[test]
    fn empty_query_or_zero_limit_returns_empty() {
        let store = SqliteL0FtsStore::in_memory().unwrap();
        store.add(&user("1", "telegram:1", "helix", 1)).unwrap();

        assert!(store.search("telegram:1", "   ", 10).unwrap().is_empty());
        assert!(store.search("telegram:1", "helix", 0).unwrap().is_empty());
    }

    #[test]
    fn sanitizer_removes_fts_syntax_punctuation() {
        assert_eq!(sanitized_fts_query("helix OR config"), Some("helix* or* config*".to_string()));
        assert_eq!(sanitized_fts_query("!!!"), None);
    }
}
```

- [ ] **Step 3: Run tests to verify failure**

Run:

```bash
cargo test l0::fts_store -- --nocapture
```

Expected: FAIL with `not yet implemented` from the new store methods.

- [ ] **Step 4: Implement schema and store methods**

Replace the `todo!` methods and add helpers in `src/l0/fts_store.rs`:

```rust
impl SqliteL0FtsStore {
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS l0_records (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                telegram_chat_id INTEGER NOT NULL,
                telegram_user_id INTEGER,
                telegram_message_id INTEGER,
                role TEXT NOT NULL,
                source TEXT NOT NULL,
                content TEXT NOT NULL,
                provider TEXT,
                model TEXT,
                tool_name TEXT,
                tool_call_id TEXT,
                raw_json TEXT,
                record_json TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_l0_records_conversation_created
                ON l0_records(conversation_id, created_at_ms);

            CREATE VIRTUAL TABLE IF NOT EXISTS l0_records_fts USING fts5(
                id UNINDEXED,
                conversation_id UNINDEXED,
                content
            );
            "#,
        )?;
        Ok(())
    }

    pub fn add(&self, record: &L0Record) -> Result<()> {
        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        let role = serde_json::to_string(&record.role)?.trim_matches('"').to_string();
        let source = serde_json::to_string(&record.source)?.trim_matches('"').to_string();
        let raw_json = record.raw_json.as_ref().map(serde_json::to_string).transpose()?;
        let record_json = serde_json::to_string(record)?;

        conn.execute(
            r#"
            INSERT OR REPLACE INTO l0_records (
                id, conversation_id, telegram_chat_id, telegram_user_id, telegram_message_id,
                role, source, content, provider, model, tool_name, tool_call_id,
                raw_json, record_json, created_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
            params![
                record.id,
                record.conversation_id,
                record.telegram_chat_id,
                record.telegram_user_id.map(|value| value as i64),
                record.telegram_message_id,
                role,
                source,
                record.content,
                record.provider,
                record.model,
                record.tool_name,
                record.tool_call_id,
                raw_json,
                record_json,
                record.created_at_ms,
            ],
        )?;

        conn.execute("DELETE FROM l0_records_fts WHERE id = ?1", params![record.id])?;
        conn.execute(
            "INSERT INTO l0_records_fts(id, conversation_id, content) VALUES (?1, ?2, ?3)",
            params![record.id, record.conversation_id, record.content],
        )?;
        Ok(())
    }

    pub fn list(&self, conversation_id: &str, limit: usize) -> Result<Vec<L0Record>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        let mut stmt = conn.prepare(
            r#"
            SELECT record_json
            FROM l0_records
            WHERE conversation_id = ?1
            ORDER BY created_at_ms DESC
            LIMIT ?2
            "#,
        )?;
        let rows = stmt.query_map(params![conversation_id, limit as i64], |row| row.get::<_, String>(0))?;
        let mut records = Vec::new();
        for row in rows {
            records.push(serde_json::from_str::<L0Record>(&row?)?);
        }
        records.reverse();
        Ok(records)
    }

    pub fn search(&self, conversation_id: &str, query: &str, limit: usize) -> Result<Vec<L0Record>> {
        let normalized = query.trim().to_ascii_lowercase();
        if normalized.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let mut seen = std::collections::HashSet::new();
        let mut results = Vec::new();

        for record in self.exact_matches(conversation_id, &normalized, limit)? {
            seen.insert(record.id.clone());
            results.push(record);
        }

        if results.len() < limit {
            for record in self.fts_matches(conversation_id, query, limit * 3)? {
                if seen.insert(record.id.clone()) {
                    results.push(record);
                    if results.len() == limit {
                        break;
                    }
                }
            }
        }

        Ok(results)
    }

    fn exact_matches(&self, conversation_id: &str, normalized_query: &str, limit: usize) -> Result<Vec<L0Record>> {
        let listed = self.list(conversation_id, usize::MAX)?;
        let mut matches = listed
            .into_iter()
            .filter(|record| record.content.to_ascii_lowercase().contains(normalized_query))
            .collect::<Vec<_>>();
        matches.sort_by_key(|record| std::cmp::Reverse(record.created_at_ms));
        matches.truncate(limit);
        Ok(matches)
    }

    fn fts_matches(&self, conversation_id: &str, query: &str, limit: usize) -> Result<Vec<L0Record>> {
        let Some(fts_query) = sanitized_fts_query(query) else {
            return Ok(Vec::new());
        };

        let conn = self.conn.lock().expect("sqlite mutex poisoned");
        let mut stmt = conn.prepare(
            r#"
            SELECT r.record_json
            FROM l0_records_fts
            JOIN l0_records r ON r.id = l0_records_fts.id
            WHERE l0_records_fts MATCH ?1
              AND r.conversation_id = ?2
            ORDER BY bm25(l0_records_fts), r.created_at_ms DESC
            LIMIT ?3
            "#,
        )?;
        let rows = stmt.query_map(params![fts_query, conversation_id, limit as i64], |row| row.get::<_, String>(0))?;
        let mut records = Vec::new();
        for row in rows {
            records.push(serde_json::from_str::<L0Record>(&row?)?);
        }
        Ok(records)
    }
}

fn sanitized_fts_query(query: &str) -> Option<String> {
    let terms = query
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| format!("{}*", term.to_ascii_lowercase()))
        .collect::<Vec<_>>();

    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}
```

- [ ] **Step 5: Run FTS store tests**

Run:

```bash
cargo test l0::fts_store -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Run existing L0 tests**

Run:

```bash
cargo test l0:: -- --nocapture
```

Expected: PASS. Existing substring tests should remain unchanged.

- [ ] **Step 7: Checkpoint**

Run:

```bash
git status --short
```

Expected: includes new `src/l0/fts_store.rs` and modified `src/l0/mod.rs`. Do not commit unless the user explicitly asks.

---

### Task 3: Add iii L0 FTS worker binary

**Files:**
- Create: `src/bin/l0_fts_worker.rs`

- [ ] **Step 1: Create worker binary**

Create `src/bin/l0_fts_worker.rs`:

```rust
mod agents;
mod config;
mod error;
mod health;
mod l0;
mod logging;
mod telegram;
mod types;

use crate::l0::fts_store::SqliteL0FtsStore;
use crate::l0::model::L0Record;
use anyhow::Context;
use iii_sdk::{register_worker, InitOptions, RegisterFunction, StreamSetInput, TriggerRequest};
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

    iii.register_function(RegisterFunction::new_async("l0::add", {
        let iii = iii.clone();
        let store = store.clone();
        let stream_name = stream_name.clone();
        move |payload: Value| {
            let iii = iii.clone();
            let store = store.clone();
            let stream_name = stream_name.clone();
            async move {
                let request: AddRequest = serde_json::from_value(payload).context("invalid l0::add payload")?;
                let record = request.record;

                let stream_payload = serde_json::to_value(StreamSetInput {
                    stream_name,
                    group_id: record.conversation_id.clone(),
                    item_id: record.id.clone(),
                    data: serde_json::to_value(&record)?,
                })?;
                iii.trigger(TriggerRequest {
                    function_id: "stream::set".to_string(),
                    payload: stream_payload,
                    action: None,
                    timeout_ms: None,
                })
                .await
                .context("l0::add stream::set failed")?;

                store.add(&record).context("l0::add sqlite index failed")?;
                Ok(json!({ "ok": true }))
            }
        }
    }));

    iii.register_function(RegisterFunction::new_async("l0::list", {
        let store = store.clone();
        move |payload: Value| {
            let store = store.clone();
            async move {
                let request: ListRequest = serde_json::from_value(payload).context("invalid l0::list payload")?;
                let records = store
                    .list(&request.conversation_id, request.limit.unwrap_or(10))
                    .context("l0::list sqlite list failed")?;
                Ok(json!({ "ok": true, "records": records }))
            }
        }
    }));

    iii.register_function(RegisterFunction::new_async("l0::search", {
        let store = store.clone();
        move |payload: Value| {
            let store = store.clone();
            async move {
                let request: SearchRequest = serde_json::from_value(payload).context("invalid l0::search payload")?;
                let records = store
                    .search(&request.conversation_id, &request.query, request.limit.unwrap_or(10))
                    .context("l0::search hybrid search failed")?;
                Ok(json!({ "ok": true, "results": records }))
            }
        }
    }));

    println!("L0 FTS worker registered l0::add, l0::list, l0::search");
    tokio::signal::ctrl_c().await?;
    iii.shutdown_async().await;
    Ok(())
}
```

If `mod agents;` style module declarations fail from `src/bin`, replace the first eight `mod ...;` lines with imports from a library crate after performing the standard Rust split: create `src/lib.rs` containing the existing module declarations and change `src/main.rs`/worker imports to `use bot::...`. Only do that split if compilation requires it.

- [ ] **Step 2: Run binary check**

Run:

```bash
cargo check --bin l0_fts_worker
```

Expected: PASS. If it fails because the binary cannot access crate modules, perform the `src/lib.rs` split described in Step 1, then re-run.

- [ ] **Step 3: Checkpoint**

Run:

```bash
git status --short
```

Expected: includes new `src/bin/l0_fts_worker.rs` and possibly `src/lib.rs`/adjusted imports if the module split was needed. Do not commit unless the user explicitly asks.

---

### Task 4: Update `IiiL0Repository` to call worker functions with fallback

**Files:**
- Modify: `src/l0/iii_repository.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add repository fields and builder**

In `src/l0/iii_repository.rs`, add a `use_worker_functions` field:

```rust
#[derive(Clone)]
pub struct IiiL0Repository {
    iii: III,
    stream_name: String,
    timeout_ms: Option<u64>,
    use_worker_functions: bool,
}
```

Update `from_client`:

```rust
pub fn from_client(iii: III, stream_name: impl Into<String>) -> Self {
    Self {
        iii,
        stream_name: stream_name.into(),
        timeout_ms: None,
        use_worker_functions: false,
    }
}
```

Add a builder:

```rust
pub fn with_worker_functions(mut self, enabled: bool) -> Self {
    self.use_worker_functions = enabled;
    self
}
```

- [ ] **Step 2: Extract existing stream methods**

Still in `src/l0/iii_repository.rs`, add helper methods using the existing logic:

```rust
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
```

- [ ] **Step 3: Add worker call helpers**

Add these helpers below the stream helpers:

```rust
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
```

Add parser:

```rust
fn records_from_l0_worker_value(mut value: serde_json::Value, field: &str) -> Result<Vec<L0Record>> {
    if let Some(object) = value.as_object_mut() {
        if let Some(records) = object.remove(field).or_else(|| object.remove("records")) {
            return records_from_stream_list_value(records);
        }
    }
    records_from_stream_list_value(value)
}
```

- [ ] **Step 4: Update trait implementation with fallback**

Replace `add`, `list`, and `search` in the `impl L0Repository for IiiL0Repository` block:

```rust
async fn add(&self, record: L0Record) -> Result<()> {
    if self.use_worker_functions {
        if self.worker_add(&record).await.is_ok() {
            return Ok(());
        }
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
```

- [ ] **Step 5: Wire config in `main.rs`**

Modify `build_l0_repository` in `src/main.rs`:

```rust
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
```

- [ ] **Step 6: Run repository tests/check**

Run:

```bash
cargo test l0::iii_repository -- --nocapture
cargo check
```

Expected: tests PASS and `cargo check` PASS. If existing tests instantiate `IiiL0Repository::from_client`, ensure the new `use_worker_functions: false` default keeps them passing.

- [ ] **Step 7: Checkpoint**

Run:

```bash
git status --short
```

Expected: includes modified `src/l0/iii_repository.rs` and `src/main.rs`. Do not commit unless the user explicitly asks.

---

### Task 5: Adapt AI tool and prompt wording for hybrid search

**Files:**
- Modify: `src/agents/tools.rs`
- Modify: `src/agents/prompts.rs`

- [ ] **Step 1: Add a tool test proving `l0_search` returns repository search results**

In `src/agents/tools.rs`, inside the existing `#[cfg(test)] mod tests`, add:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn registered_l0_tools_can_search_current_conversation() {
    let repo = Arc::new(MemoryL0Repository::new());
    repo.add(L0Record::new_user(
        "id-1".to_string(),
        "telegram:1".to_string(),
        1,
        Some(2),
        Some(3),
        "my favorite editor is Helix".to_string(),
        1,
    ))
    .await
    .unwrap();
    repo.add(L0Record::new_user(
        "id-2".to_string(),
        "telegram:2".to_string(),
        2,
        Some(2),
        Some(3),
        "Helix in another chat".to_string(),
        2,
    ))
    .await
    .unwrap();

    let tools = l0_tools(
        TelegramMeta::from_chat(1, Some(2), Some(3)),
        repo,
        Arc::new(LoggingBus::default()),
    );
    let search_tool = tools.iter().find(|tool| tool.name == "l0_search").unwrap();

    let output = search_tool
        .execute
        .call(serde_json::json!({ "query": "favorite editor", "limit": 10 }))
        .unwrap();

    assert!(output.contains("my favorite editor is Helix"));
    assert!(!output.contains("another chat"));
}
```

- [ ] **Step 2: Run tool test**

Run:

```bash
cargo test agents::tools::tests::registered_l0_tools_can_search_current_conversation -- --nocapture
```

Expected: PASS with current repository because the test uses an exact phrase. This guards the tool path while backend search changes.

- [ ] **Step 3: Update tool description**

In `src/agents/tools.rs`, change the `l0_search` description from:

```rust
description: "Search raw L0 records in the current Telegram conversation.".to_string(),
```

to:

```rust
description: "Hybrid keyword/phrase search over raw L0 record content in the current Telegram conversation.".to_string(),
```

Do not add `mode`, `scope`, or write fields to `L0SearchInput`.

- [ ] **Step 4: Update prompt wording**

In `src/agents/prompts.rs`, update the memory search instructions:

Replace:

```text
- Use l0_search for older or specific facts, such as a prior preference, name, repeated question, or earlier tool result.
```

with:

```text
- Use l0_search for older or specific facts, such as a prior preference, name, repeated question, or earlier tool result. It performs hybrid keyword/phrase search over L0 content for the current conversation.
```

Replace:

```text
- l0_search: search raw L0 records for the current Telegram conversation.
```

with:

```text
- l0_search: hybrid keyword/phrase search over raw L0 record content for the current Telegram conversation.
```

- [ ] **Step 5: Run targeted tests**

Run:

```bash
cargo test agents::tools -- --nocapture
cargo test agents::prompts -- --nocapture
```

Expected: PASS. If prompt tests assert exact text, update them to assert key behavior rather than full snapshots, consistent with `CLAUDE.md`.

- [ ] **Step 6: Checkpoint**

Run:

```bash
git status --short
```

Expected: includes modified `src/agents/tools.rs` and `src/agents/prompts.rs`. Do not commit unless the user explicitly asks.

---

### Task 6: Document operation and fallback behavior

**Files:**
- Modify: `README.md` or `DEV.md`

- [ ] **Step 1: Add docs section**

Add this section to `DEV.md` unless the repository already has a better runtime operations section in `README.md`:

```markdown
## L0 hybrid FTS search

L0 search can run in two modes:

1. Default mode: the bot uses `iii-stream` directly and performs local case-insensitive substring search over `L0Record.content`.
2. Worker mode: the bot calls custom iii functions `l0::add`, `l0::list`, and `l0::search`. The worker stores raw L0 records in `iii-stream` and indexes `L0Record.content` in SQLite FTS.

Enable worker mode:

```dotenv
L0_USE_WORKER_FUNCTIONS=true
L0_FTS_SQLITE_PATH=./data/iii.db
```

Run the worker in a separate terminal after iii is running:

```bash
cargo run --bin l0_fts_worker
```

Run the bot separately after the worker is registered. If `l0::search` fails, the bot falls back to the existing `stream::list` + substring search path. Search remains scoped to the current Telegram conversation and only searches L0 `content`, not `raw_json` or metadata.
```

Do not document real secrets.

- [ ] **Step 2: Run docs-independent check**

Run:

```bash
cargo check
```

Expected: PASS.

- [ ] **Step 3: Checkpoint**

Run:

```bash
git status --short
```

Expected: includes docs changes. Do not commit unless the user explicitly asks.

---

### Task 7: Full verification

**Files:**
- No new files; verify all changed modules.

- [ ] **Step 1: Run targeted L0 tests**

Run:

```bash
cargo test l0:: -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run targeted tool/prompt tests**

Run:

```bash
cargo test agents::tools -- --nocapture
cargo test agents::prompts -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run binary check**

Run:

```bash
cargo check --bin bot
cargo check --bin l0_fts_worker
```

Expected: both PASS. If `cargo check --bin bot` fails because the package binary name is not `bot`, run `cargo check` and record the actual binary name from Cargo's error.

- [ ] **Step 4: Run full test suite**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 5: Optional manual iii verification only with user approval**

Because `CLAUDE.md` says to ask before running the bot locally, ask the user before running long-lived processes. If approved, use two terminals or background tasks:

```bash
cargo run --bin l0_fts_worker
```

Then run the bot only if the user explicitly approves:

```bash
cargo run
```

Manual checks in Telegram:

```text
/l0search favorite editor
/l0search helix config
```

Expected: exact phrase searches still work, and tokenized keyword searches can match content where words are separated or inflected by prefix matching.

- [ ] **Step 6: Final checkpoint**

Run:

```bash
git status --short
```

Expected: shows all changed files. Report tests run and exact results. Do not say the feature is complete unless the verification commands passed; do not commit unless the user explicitly asks.

---

## Self-review

- Spec coverage: the plan covers iii worker functions, SQLite FTS indexing, hybrid exact+FTS search, Telegram and AI tool reuse through `L0Repository::search`, fallback behavior, docs, and tests.
- Placeholder scan: no implementation step uses `TBD`, `TODO`, or vague “handle errors” language. The only `todo!` calls appear intentionally in the failing-test step and are replaced in the next step.
- Type consistency: all new public names are consistent across tasks: `SqliteL0FtsStore`, `l0_use_worker_functions`, `l0_fts_sqlite_path`, `l0::add`, `l0::list`, and `l0::search`.
- Scope: embeddings, metadata search, user-facing search modes, health output changes, and AI-visible write tools are explicitly excluded.
