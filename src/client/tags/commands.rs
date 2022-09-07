use serenity::{
    builder::{CreateApplicationCommand, CreateApplicationCommandOption},
    model::prelude::command::CommandOptionType,
};
use strum::EnumProperty;

use crate::client::slash_commands::UserCommand;

pub fn create_tag_commands() -> CreateApplicationCommand {
    let command = CreateApplicationCommand::new("tag");

    command
        .dm_permission(false)
        .description("tagdesc")
        .add_option(create_tag_creation_option())
        .add_option(create_tag_removal_option())
        .add_option(CreateApplicationCommandOption::new(
            CommandOptionType::SubCommand,
            UserCommand::taglist.get_str("SubCommand").unwrap(),
            "List all of the tags",
        ))
        .add_option(CreateApplicationCommandOption::new(
            CommandOptionType::SubCommand,
            UserCommand::blacklistmefromtags
                .get_str("SubCommand")
                .unwrap(),
            "The bot won't respond to your messages if you trip off a tag",
        ))
        .add_option(CreateApplicationCommandOption::new(
            CommandOptionType::SubCommand,
            UserCommand::tagresponsechannel
                .get_str("SubCommand")
                .unwrap(),
            "Set this channel as the channel where i will reply to tags",
        ))
}

fn create_tag_creation_option() -> CreateApplicationCommandOption {
    CreateApplicationCommandOption::new(
        CommandOptionType::SubCommand,
        UserCommand::createtag.get_str("SubCommand").unwrap(),
        "Create a tag for a word or list of words and a response whenever someone says that word",
    )
    .add_sub_option(
        CreateApplicationCommandOption::new(
            CommandOptionType::String,
            "tag",
            "What word to listen for",
        )
        .required(true),
    )
    .add_sub_option(
        CreateApplicationCommandOption::new(
            CommandOptionType::String,
            "response",
            "What the response should be when the tag is said",
        )
        .required(true),
    )
}

fn create_tag_removal_option() -> CreateApplicationCommandOption {
    CreateApplicationCommandOption::new(
        CommandOptionType::SubCommand,
        UserCommand::removetag.get_str("SubCommand").unwrap(),
        "Remove a tag",
    )
    .add_sub_option(
        CreateApplicationCommandOption::new(CommandOptionType::String, "tag", "The tag to remove")
            .required(true),
    )
}
