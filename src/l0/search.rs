use crate::l0::model::L0Record;

pub fn search_records(records: &[L0Record], query: &str, limit: usize) -> Vec<L0Record> {
    let normalized = query.trim().to_ascii_lowercase();
    if normalized.is_empty() || limit == 0 {
        return Vec::new();
    }

    records
        .iter()
        .filter(|record| record.content.to_ascii_lowercase().contains(&normalized))
        .rev()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l0::model::{L0Record, L0Role, L0Source};

    fn record(id: &str, content: &str, created_at_ms: i64) -> L0Record {
        L0Record {
            id: id.to_string(),
            conversation_id: "telegram:1".to_string(),
            telegram_chat_id: 1,
            telegram_user_id: None,
            telegram_message_id: None,
            role: L0Role::User,
            content: content.to_string(),
            source: L0Source::TelegramUpdate,
            provider: None,
            model: None,
            tool_name: None,
            tool_call_id: None,
            raw_json: None,
            created_at_ms,
        }
    }

    #[test]
    fn searches_case_insensitive_and_limits_results() {
        let records = vec![
            record("1", "I like Helix", 1),
            record("2", "Other message", 2),
            record("3", "helix config", 3),
        ];
        let result = search_records(&records, "HELIX", 1);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "3");
    }
}
