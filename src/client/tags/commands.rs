use serenity::{
    all::{CommandOptionType, InteractionContext},
    builder::{CreateCommand, CreateCommandOption},
};
use strum::EnumProperty;

use crate::client::slash_commands::UserCommand;

pub fn create_tag_commands() -> CreateCommand {
    let command = CreateCommand::new("tag");

    command
        .add_context(InteractionContext::Guild)
        .description("tagdesc")
        .add_option(create_tag_creation_option())
        .add_option(create_tag_removal_option())
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            UserCommand::tag_list.get_str("SubCommand").unwrap(),
            "List all of the tags",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            UserCommand::blacklist_me_from_tags
                .get_str("SubCommand")
                .unwrap(),
            "The bot won't respond to your messages if you trip off a tag",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            UserCommand::tag_response_channel
                .get_str("SubCommand")
                .unwrap(),
            "Set this channel as the channel where i will reply to tags",
        ))
}

fn create_tag_creation_option() -> CreateCommandOption {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        UserCommand::create_tag.get_str("SubCommand").unwrap(),
        "Create a tag for a word or list of words and a response whenever someone says that word",
    )
    .add_sub_option(
        CreateCommandOption::new(CommandOptionType::String, "tag", "What word to listen for")
            .required(true),
    )
    .add_sub_option(
        CreateCommandOption::new(
            CommandOptionType::String,
            "response",
            "What the response should be when the tag is said",
        )
        .required(true),
    )
}

fn create_tag_removal_option() -> CreateCommandOption {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        UserCommand::remove_tag.get_str("SubCommand").unwrap(),
        "Remove a tag",
    )
    .add_sub_option(
        CreateCommandOption::new(CommandOptionType::String, "tag", "The tag to remove")
            .required(true),
    )
}
