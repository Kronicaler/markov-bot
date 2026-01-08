use serenity::all::{
    CommandOptionType, CommandType, CreateCommand, CreateCommandOption, InstallationContext,
    InteractionContext,
};
use strum::EnumProperty;

use crate::client::slash_commands::UserCommand;

pub fn create_memes_commands() -> Vec<CreateCommand> {
    let upload_meme_command = CreateCommand::new(UserCommand::upload_meme.to_string())
        .add_integration_type(InstallationContext::User)
        .add_integration_type(InstallationContext::Guild)
        .add_context(InteractionContext::Guild)
        .add_context(InteractionContext::PrivateChannel)
        .kind(CommandType::Message);

    let post_meme_command = CreateCommand::new("meme")
        .description("description")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                UserCommand::post_meme.get_str("SubCommand").unwrap(),
                "Post a meme from a desired tag",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "tag", "Select a tag")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "ordered",
                    "send from oldest to newest in this server for this tag",
                )
                .required(false),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "ephemeral",
                    "send the meme in an ephemeral message only you can see",
                )
                .required(false),
            ),
        );

    vec![upload_meme_command, post_meme_command]
}
