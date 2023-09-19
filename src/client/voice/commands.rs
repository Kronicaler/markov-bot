use serenity::{
    builder::{CreateApplicationCommand, CreateApplicationCommandOption},
    model::prelude::command::CommandOptionType,
};

use crate::client::slash_commands::UserCommand;

pub fn create_voice_commands() -> Vec<CreateApplicationCommand> {
    vec![
        create_play_command(),
        create_skip_command(),
        create_swap_command(),
        CreateApplicationCommand::new(UserCommand::stop.to_string())
            .description("stop playing and clear the queue"),
        CreateApplicationCommand::new(UserCommand::playing.to_string())
            .description("get info for current song"),
        CreateApplicationCommand::new(UserCommand::queue.to_string())
            .description("get the current queue"),
        CreateApplicationCommand::new(UserCommand::queue_shuffle.to_string())
            .description("shuffle all the songs in the queue"),
        CreateApplicationCommand::new(UserCommand::loop_song.to_string())
            .description("loop the current "),
    ]
}

fn create_play_command() -> CreateApplicationCommand {
    CreateApplicationCommand::new(UserCommand::play.to_string())
        .description("play song from youtube")
        .add_option(
            CreateApplicationCommandOption::new(
                CommandOptionType::String,
                "query",
                "what to search youtube for",
            )
            .required(true),
        )
}

fn create_skip_command() -> CreateApplicationCommand {
    CreateApplicationCommand::new(UserCommand::skip.to_string())
        .description("skip one or multiple songs")
        .add_option(
            CreateApplicationCommandOption::new(
                CommandOptionType::Integer,
                "number",
                "skip the requested song in the queue",
            )
            .required(false),
        )
        .add_option(
            CreateApplicationCommandOption::new(
                CommandOptionType::Integer,
                "until",
                "skip all the songs before the requested song",
            )
            .required(false),
        )
}

fn create_swap_command() -> CreateApplicationCommand {
    CreateApplicationCommand::new(UserCommand::swap_songs.to_string())
        .description("swap the positions of 2 songs in the queue")
        .add_option(
            CreateApplicationCommandOption::new(
                CommandOptionType::Integer,
                "first-track",
                "The first track to swap",
            )
            .required(true),
        )
        .add_option(
            CreateApplicationCommandOption::new(
                CommandOptionType::Integer,
                "second-track",
                "The second track to swap",
            )
            .required(true),
        )
}
