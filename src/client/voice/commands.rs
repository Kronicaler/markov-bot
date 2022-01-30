use serenity::{
    builder::CreateApplicationCommands,
    model::interactions::application_command::ApplicationCommandOptionType,
};

use crate::client::slash_commands::Command;

pub trait VoiceCommandBuilder {
    fn create_voice_commands(&mut self) -> &mut Self;
}

impl VoiceCommandBuilder for CreateApplicationCommands {
    fn create_voice_commands(&mut self) -> &mut Self {
        self.create_application_command(|command| {
            command
                .name(Command::play)
                .description("play song from youtube")
                .create_option(|option| {
                    option
                        .name("query")
                        .description("what to search youtube for")
                        .kind(ApplicationCommandOptionType::String)
                        .required(true)
                })
        })
        //skip a song
        .create_application_command(|command| {
            command
                .name(Command::skip)
                .description("skip the current song")
                .create_option(|option| {
                    option
                        .name("number")
                        .description("Number in queue")
                        .kind(ApplicationCommandOptionType::Integer)
                        .required(false)
                })
        })
        //stop playing
        .create_application_command(|command| {
            command
                .name(Command::stop)
                .description("stop playing and clear the queue")
        })
        //get info of current song
        .create_application_command(|command| {
            command
                .name(Command::playing)
                .description("get info for current song")
        })
        //get queue
        .create_application_command(|command| {
            command
                .name(Command::queue)
                .description("get the current queue")
        })
    }
}
