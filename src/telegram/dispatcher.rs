use crate::telegram::commands::Command;
use crate::telegram::handlers::{handle_callback_query, handle_command, handle_text, BotState};
use std::sync::Arc;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;

pub async fn run(bot: Bot, state: Arc<BotState>) {
    let message_handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(handle_command),
        )
        .branch(Message::filter_text().endpoint(handle_text));

    let handler = dptree::entry()
        .branch(message_handler)
        .branch(Update::filter_callback_query().endpoint(handle_callback_query));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
