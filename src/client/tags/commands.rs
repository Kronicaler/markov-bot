use serenity::{
    builder::{CreateApplicationCommand, CreateApplicationCommands},
    model::prelude::command::CommandOptionType,
};
use strum::EnumProperty;

use crate::client::slash_commands::UserCommand;

/// Create the tag slash commands
pub trait TagCommandBuilder {
    fn create_tag_commands(&mut self) -> &mut Self;
}

pub trait TagCommandOptions {
    fn create_tag_creation_option(&mut self) -> &mut Self;
    fn create_tag_removal_option(&mut self) -> &mut Self;
}

impl TagCommandBuilder for CreateApplicationCommands {
    fn create_tag_commands(&mut self) -> &mut Self {
        let mut command = CreateApplicationCommand::default();

        command
            .name("tag")
            .dm_permission(false)
            .description("tagdesc")
            .create_tag_creation_option()
            .create_tag_removal_option()
            .create_option(|command| {
                command
                    .name(UserCommand::taglist.get_str("SubCommand").unwrap())
                    .kind(CommandOptionType::SubCommand)
                    .description("List all of the tags")
            })
            .create_option(|command| {
                command
                    .name(
                        UserCommand::blacklistmefromtags
                            .get_str("SubCommand")
                            .unwrap(),
                    )
                    .kind(CommandOptionType::SubCommand)
                    .description("The bot won't respond to your messages if you trip off a tag")
            })
            .create_option(|command| {
                command
                    .name(
                        UserCommand::tagresponsechannel
                            .get_str("SubCommand")
                            .unwrap(),
                    )
                    .kind(CommandOptionType::SubCommand)
                    .description("Set this channel as the channel where i will reply to tags")
            });

        self.add_application_command(command)
    }
}

impl TagCommandOptions for CreateApplicationCommand {
    fn create_tag_creation_option(&mut self) -> &mut Self {
        self.create_option(|command|
            command
            .name(UserCommand::createtag.get_str("SubCommand").unwrap())
            .kind(CommandOptionType::SubCommand)
            .description("Create a tag for a word or list of words and a response whenever someone says that word")
            .create_sub_option(|option|
                option
                .name("tag")
                .description("What word to listen for")
                .kind(CommandOptionType::String)
                .required(true)
            )
            .create_sub_option(|option|
                option
                .name("response")
                .description("What the response should be when the tag is said")
                .kind(CommandOptionType::String)
                .required(true)
            )
        )
    }

    fn create_tag_removal_option(&mut self) -> &mut Self {
        self.create_option(|command| {
            command
                .name(UserCommand::removetag.get_str("SubCommand").unwrap())
                .kind(CommandOptionType::SubCommand)
                .description("Remove a tag")
                .create_sub_option(|option| {
                    option
                        .name("tag")
                        .description("The tag to remove")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
        })
    }
}
