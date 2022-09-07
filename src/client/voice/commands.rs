use serenity::{model::prelude::command::CommandOptionType, builder::{CreateApplicationCommand, CreateApplicationCommandOption}};

use crate::client::slash_commands::UserCommand;

    pub fn create_voice_commands() -> Vec<CreateApplicationCommand> {
        return vec![
            create_play_command(),
            create_skip_command(),
            create_swap_command(),

        ];

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

    fn create_play_command() -> CreateApplicationCommand {
        CreateApplicationCommand::new(
            UserCommand::play.to_string())
            .description("play song from youtube")
            .add_option(CreateApplicationCommandOption::new(CommandOptionType::String, "query", "what to search youtube for").required(true)
        )
    }

    fn create_skip_command() -> CreateApplicationCommand {
        CreateApplicationCommand::new(UserCommand::skip.to_string())
        .description("skip the current song")
        .add_option(CreateApplicationCommandOption::new(CommandOptionType::Integer, "number", "Number in queue").required(false)
        )
    }

    fn create_swap_command() -> CreateApplicationCommand {
        CreateApplicationCommand::new(UserCommand::swap_songs)
        .description("swap the positions of 2 songs in the queue")
        .add_option(CreateApplicationCommandOption::new(CommandOptionType::Integer)
                .name("first-track")
                .description("The first track to swap")
                .required(true)
                .kind(CommandOptionType::Integer)
        )


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
