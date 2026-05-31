use crate::error::Result;
use crate::l0::model::L0Record;
use crate::l0::repository::L0Repository;
use crate::l0::search::search_records;
use async_trait::async_trait;
use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct MemoryL0Repository {
    records: RwLock<Vec<L0Record>>,
}

impl MemoryL0Repository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl L0Repository for MemoryL0Repository {
    async fn add(&self, record: L0Record) -> Result<()> {
        self.records.write().await.push(record);
        Ok(())
    }

    async fn list(&self, conversation_id: &str, limit: usize) -> Result<Vec<L0Record>> {
        let records = self.records.read().await;
        let mut filtered = records
            .iter()
            .filter(|record| record.conversation_id == conversation_id)
            .cloned()
            .collect::<Vec<_>>();
        filtered.sort_by_key(|record| record.created_at_ms);
        let start = filtered.len().saturating_sub(limit);
        Ok(filtered[start..].to_vec())
    }

    async fn search(&self, conversation_id: &str, query: &str, limit: usize) -> Result<Vec<L0Record>> {
        let listed = self.list(conversation_id, usize::MAX).await?;
        Ok(search_records(&listed, query, limit))
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
            None,
            None,
            content.to_string(),
            created_at_ms,
        )
    }

    #[tokio::test]
    async fn lists_by_conversation_and_limit() {
        let repo = MemoryL0Repository::new();
        repo.add(user("1", "telegram:1", "one", 1)).await.unwrap();
        repo.add(user("2", "telegram:2", "two", 2)).await.unwrap();
        repo.add(user("3", "telegram:1", "three", 3)).await.unwrap();

        let records = repo.list("telegram:1", 1).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "3");
    }

    #[tokio::test]
    async fn searches_within_conversation() {
        let repo = MemoryL0Repository::new();
        repo.add(user("1", "telegram:1", "helix editor", 1)).await.unwrap();
        repo.add(user("2", "telegram:2", "helix other chat", 2)).await.unwrap();

        let records = repo.search("telegram:1", "helix", 10).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "1");
    }
}
