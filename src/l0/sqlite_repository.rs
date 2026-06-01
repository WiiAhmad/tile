use crate::error::Result;
use crate::l0::fts_store::SqliteL0FtsStore;
use crate::l0::model::L0Record;
use crate::l0::repository::L0Repository;
use async_trait::async_trait;

#[derive(Clone)]
pub struct SqliteL0Repository {
    store: SqliteL0FtsStore,
}

impl SqliteL0Repository {
    pub fn from_store(store: SqliteL0FtsStore) -> Self {
        Self { store }
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        Ok(Self::from_store(SqliteL0FtsStore::in_memory()?))
    }
}

#[async_trait]
impl L0Repository for SqliteL0Repository {
    async fn add(&self, record: L0Record) -> Result<()> {
        self.store.add(&record)
    }

    async fn list(&self, conversation_id: &str, limit: usize) -> Result<Vec<L0Record>> {
        self.store.list(conversation_id, limit)
    }

    async fn search(&self, conversation_id: &str, query: &str, limit: usize) -> Result<Vec<L0Record>> {
        self.store.search(conversation_id, query, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    async fn adds_lists_and_searches_records() {
        let repo = SqliteL0Repository::in_memory().unwrap();
        repo.add(user("1", "telegram:1", "my favorite editor is helix", 1)).await.unwrap();
        repo.add(user("2", "telegram:2", "helix other chat", 2)).await.unwrap();
        repo.add(user("3", "telegram:1", "editor notes", 3)).await.unwrap();

        let listed = repo.list("telegram:1", 10).await.unwrap();
        assert_eq!(listed.iter().map(|record| record.id.as_str()).collect::<Vec<_>>(), vec!["1", "3"]);

        let searched = repo.search("telegram:1", "favorite editor", 10).await.unwrap();
        assert_eq!(searched.first().map(|record| record.id.as_str()), Some("1"));
        assert!(searched.iter().all(|record| record.conversation_id == "telegram:1"));
    }
}
