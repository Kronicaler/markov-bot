use serenity::{
    all::{CreateCommand, InteractionContext},
    model::Permissions,
};

use crate::client::slash_commands::UserCommand;

pub fn create_markov_commands() -> Vec<CreateCommand> {
    vec![
        CreateCommand::new(UserCommand::stop_saving_my_messages.to_string())
        .description("Blacklist yourself if you don't want me to save and learn from your messages"),
        CreateCommand::new(UserCommand::stop_saving_messages_server.to_string())
        .description("Blacklist this server if you don't want me to save and learn from the messages sent in this server")
        .add_context(InteractionContext::Guild)
        .default_member_permissions(Permissions::ADMINISTRATOR),
        CreateCommand::new(UserCommand::stop_saving_messages_channel.to_string())
        .description("Blacklist this channel if you don't want me to save and learn from the messages sent in this channel")
        .add_context(InteractionContext::Guild)
        .default_member_permissions(Permissions::ADMINISTRATOR),
        CreateCommand::new(UserCommand::continue_saving_my_messages.to_string()).description(
            "Remove yourself from the blacklist if you want me to save and learn from your messages",
        )
    ]
}
