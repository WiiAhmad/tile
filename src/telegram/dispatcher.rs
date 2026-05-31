use crate::telegram::commands::Command;
use crate::telegram::handlers::{handle_command, handle_text, BotState};
use std::sync::Arc;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;

pub async fn run(bot: Bot, state: Arc<BotState>) {
    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(handle_command),
        )
        .branch(Message::filter_text().endpoint(handle_text));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
