use serenity::{
    builder::CreateApplicationCommands,
    model::interactions::application_command::ApplicationCommandOptionType,
};

use crate::client::slash_commands::Command;

/// Create the tag slash commands

pub trait TagCommandBuilder {
    fn create_tag_commands(&mut self) -> &mut Self;
}

impl TagCommandBuilder for CreateApplicationCommands {
    fn create_tag_commands(&mut self) -> &mut Self {
        self.create_application_command(|command| {
            command.name(Command::createtag).description(
                "Create a tag for a word or list of words and a response whenever someone says that word",
            )
            .create_option(|option|{
                option.name("tag").description("What word to listen for").kind(ApplicationCommandOptionType::String).required(true)
            })
            .create_option(|option|{
                option.name("response").description("What the response should be when the tag is said")
                .kind(ApplicationCommandOptionType::String)
                .required(true)
            })
        })
        .create_application_command(|command| {
            command.name(Command::removetag).description("Remove a tag").create_option(|option|{
                option.name("tag").description("The tag to remove").kind(ApplicationCommandOptionType::String).required(true)
            })
        })
        .create_application_command(|command|{
            command.name(Command::tags).description("List all of the tags")
        })
        .create_application_command(|command|{
            command.name(Command::blacklistmefromtags).description("The bot won't respond to your messages if you trip off a tag")
        })
    }
}
