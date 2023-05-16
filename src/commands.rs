use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands.")]
pub enum Command {
    #[command(description = "Show help message")]
    Help,
    #[command(description = "Begin training session")]
    Start,
    #[command(description = "Exit current session")]
    Exit,
}
