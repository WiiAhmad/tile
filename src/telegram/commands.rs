use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug, PartialEq, Eq)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "start the bot")]
    Start,
    #[command(description = "show this help text")]
    Help,
    #[command(description = "show current AI provider and model")]
    Model,
    #[command(description = "show database, iii, and L0 backend health")]
    Health,
    #[command(description = "list recent L0 records")]
    L0List,
    #[command(description = "search L0 memory")]
    L0Search(String),
}
