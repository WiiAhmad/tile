use crate::l0::model::{L0Record, L0Role};
use aisdk::core::{Message, Messages};
use std::collections::HashSet;

pub fn select_bounded_history(
    mut records: Vec<L0Record>,
    max_user: usize,
    max_assistant: usize,
) -> Vec<L0Record> {
    records.sort_by_key(|record| record.created_at_ms);

    let mut selected_ids = HashSet::new();

    for record in records.iter().rev().filter(|record| record.role == L0Role::User).take(max_user) {
        selected_ids.insert(record.id.clone());
    }

    for record in records.iter().rev().filter(|record| record.role == L0Role::Assistant).take(max_assistant) {
        selected_ids.insert(record.id.clone());
    }

    for record in records.iter().filter(|record| record.role == L0Role::System) {
        selected_ids.insert(record.id.clone());
    }

    records
        .into_iter()
        .filter(|record| selected_ids.contains(&record.id))
        .collect()
}

#[cfg(test)]
pub fn count_roles(records: &[L0Record]) -> (usize, usize) {
    let user = records.iter().filter(|record| record.role == L0Role::User).count();
    let assistant = records.iter().filter(|record| record.role == L0Role::Assistant).count();
    (user, assistant)
}

pub fn build_aisdk_history(
    records: Vec<L0Record>,
    max_user: usize,
    max_assistant: usize,
) -> Messages {
    select_bounded_history(records, max_user, max_assistant)
        .into_iter()
        .filter_map(|record| match record.role {
            L0Role::System => Some(Message::System(record.content.into())),
            L0Role::User => Some(Message::User(record.content.into())),
            L0Role::Assistant => Some(Message::Assistant(record.content.into())),
            L0Role::Tool | L0Role::Telegram => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::l0::model::{L0Role, L0Source};

    fn record(id: usize, role: L0Role) -> L0Record {
        L0Record {
            id: id.to_string(),
            conversation_id: "telegram:1".to_string(),
            telegram_chat_id: 1,
            telegram_user_id: None,
            telegram_message_id: None,
            role,
            content: format!("message {id}"),
            source: L0Source::Manual,
            provider: None,
            model: None,
            tool_name: None,
            tool_call_id: None,
            raw_json: None,
            created_at_ms: id as i64,
        }
    }

    #[test]
    fn limits_to_newest_user_and_assistant_messages() {
        let mut records = Vec::new();
        for id in 0..20 {
            records.push(record(id, L0Role::User));
        }
        for id in 20..40 {
            records.push(record(id, L0Role::Assistant));
        }

        let selected = select_bounded_history(records, 15, 15);
        let (user, assistant) = count_roles(&selected);
        assert_eq!(user, 15);
        assert_eq!(assistant, 15);
        assert!(!selected.iter().any(|record| record.id == "0"));
        assert!(!selected.iter().any(|record| record.id == "20"));
    }

    #[test]
    fn keeps_system_messages() {
        let selected = select_bounded_history(vec![record(1, L0Role::System)], 0, 0);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].role, L0Role::System);
    }

    #[test]
    fn builds_aisdk_history_from_bounded_l0_records() {
        let messages = build_aisdk_history(
            vec![
                record(1, L0Role::System),
                record(2, L0Role::User),
                record(3, L0Role::Assistant),
                record(4, L0Role::Tool),
            ],
            1,
            1,
        );

        assert_eq!(messages.len(), 3);
        assert!(matches!(messages[0], aisdk::core::Message::System(_)));
        assert!(matches!(messages[1], aisdk::core::Message::User(_)));
        assert!(matches!(messages[2], aisdk::core::Message::Assistant(_)));
    }
}
