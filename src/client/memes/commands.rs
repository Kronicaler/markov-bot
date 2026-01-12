use serenity::all::{
    CommandOptionType, CommandType, CreateCommand, CreateCommandOption, InstallationContext,
    InteractionContext,
};
use strum::EnumProperty;

use crate::client::slash_commands::UserCommand;

pub fn create_memes_commands() -> Vec<CreateCommand<'static>> {
    let upload_meme_command = CreateCommand::new(UserCommand::upload_meme.to_string())
        .add_integration_type(InstallationContext::User)
        .add_integration_type(InstallationContext::Guild)
        .add_context(InteractionContext::Guild)
        .add_context(InteractionContext::PrivateChannel)
        .kind(CommandType::Message);

    let meme_commands = CreateCommand::new("meme")
        .description("description")
        .add_integration_type(InstallationContext::User)
        .add_integration_type(InstallationContext::Guild)
        .add_context(InteractionContext::Guild)
        .add_context(InteractionContext::PrivateChannel)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                UserCommand::meme_post.get_str("SubCommand").unwrap(),
                "Post a meme from a desired tag",
            )
            .add_sub_option(
                CreateCommandOption::new(CommandOptionType::String, "tag", "Select a tag")
                    .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "random",
                    "by default memes are sent from oldest to newest in this server for this tag",
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
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            UserCommand::meme_tags.get_str("SubCommand").unwrap(),
            "See the number of memes in the most popular tags",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                UserCommand::meme_upload.get_str("SubCommand").unwrap(),
                "Upload a meme from a link",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "link",
                    "Url to the video/image",
                )
                .required(true),
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "tags",
                    "type in tags separated by spaces",
                )
                .required(true),
            ),
        );

    vec![upload_meme_command, meme_commands]
}
