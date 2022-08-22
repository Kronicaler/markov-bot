use serenity::{builder::CreateApplicationCommands, model::prelude::command::CommandOptionType};

use crate::client::slash_commands::UserCommand;

pub trait VoiceCommandBuilder {
    fn create_voice_commands(&mut self) -> &mut CreateApplicationCommands;
    fn create_play_command(&mut self) -> &mut CreateApplicationCommands;
    fn create_skip_command(&mut self) -> &mut CreateApplicationCommands;
    fn create_swap_command(&mut self) -> &mut CreateApplicationCommands;
}

impl VoiceCommandBuilder for CreateApplicationCommands {
    fn create_voice_commands(&mut self) -> &mut Self {
        self.create_play_command()
            .create_skip_command()
            .create_swap_command()
            //stop playing
            .create_application_command(|command| {
                command
                    .name(UserCommand::stop)
                    .description("stop playing and clear the queue")
            })
            //get info of current song
            .create_application_command(|command| {
                command
                    .name(UserCommand::playing)
                    .description("get info for current song")
            })
            //get queue
            .create_application_command(|command| {
                command
                    .name(UserCommand::queue)
                    .description("get the current queue")
            })
            .create_application_command(|command| {
                command
                    .name(UserCommand::loop_song)
                    .description("loop the current song")
            })
    }

    fn create_play_command(&mut self) -> &mut CreateApplicationCommands {
        self.create_application_command(|command| {
            command
                .name(UserCommand::play)
                .description("play song from youtube")
                .create_option(|option| {
                    option
                        .name("query")
                        .description("what to search youtube for")
                        .kind(CommandOptionType::String)
                        .required(true)
                })
        })
    }

    fn create_skip_command(&mut self) -> &mut CreateApplicationCommands {
        self.create_application_command(|command| {
            command
                .name(UserCommand::skip)
                .description("skip the current song")
                .create_option(|option| {
                    option
                        .name("number")
                        .description("Number in queue")
                        .kind(CommandOptionType::Integer)
                        .required(false)
                })
        })
    }

    fn create_swap_command(&mut self) -> &mut CreateApplicationCommands {
        self.create_application_command(|command| {
            command
                .name(UserCommand::swap_songs)
                .description("swap the positions of 2 songs in the queue")
                .create_option(|option| {
                    option
                        .name("first-track")
                        .description("The first track to swap")
                        .required(true)
                        .kind(CommandOptionType::Integer)
                })
                .create_option(|option| {
                    option
                        .name("second-track")
                        .description("The second track to swap")
                        .required(true)
                        .kind(CommandOptionType::Integer)
                })
        })
    }
}
