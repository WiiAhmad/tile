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
                &record.id,
                &record.conversation_id,
                record.telegram_chat_id,
                record.telegram_user_id.map(|value| value as i64),
                record.telegram_message_id,
                role,
                source,
                &record.content,
                &record.provider,
                &record.model,
                &record.tool_name,
                &record.tool_call_id,
                raw_json,
                record_json,
                record.created_at_ms,
            ],
        )?;

        conn.execute("DELETE FROM l0_records_fts WHERE id = ?1", params![&record.id])?;
        conn.execute(
            "INSERT INTO l0_records_fts(id, conversation_id, content) VALUES (?1, ?2, ?3)",
            params![&record.id, &record.conversation_id, &record.content],
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
        let listed = self.list(conversation_id, i64::MAX as usize)?;
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
