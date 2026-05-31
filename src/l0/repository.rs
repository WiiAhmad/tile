use crate::error::Result;
use crate::l0::model::L0Record;
use async_trait::async_trait;

#[async_trait]
pub trait L0Repository: Send + Sync {
    async fn add(&self, record: L0Record) -> Result<()>;
    async fn list(&self, conversation_id: &str, limit: usize) -> Result<Vec<L0Record>>;
    async fn search(&self, conversation_id: &str, query: &str, limit: usize) -> Result<Vec<L0Record>>;
}
