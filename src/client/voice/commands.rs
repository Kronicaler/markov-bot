use serenity::{
    all::{CommandOptionType, CommandType},
    builder::{CreateCommand, CreateCommandOption},
};

use crate::client::slash_commands::UserCommand;

pub fn create_voice_commands() -> Vec<CreateCommand<'static>> {
    vec![
        create_play_command(),
        create_play_video_command(),
        create_skip_command(),
        create_swap_command(),
        CreateCommand::new(UserCommand::stop.to_string())
            .description("stop playing and clear the queue"),
        CreateCommand::new(UserCommand::playing.to_string())
            .description("get info for current song"),
        CreateCommand::new(UserCommand::queue.to_string()).description("get the current queue"),
        CreateCommand::new(UserCommand::queue_shuffle.to_string())
            .description("shuffle all the songs in the queue"),
        CreateCommand::new(UserCommand::loop_song.to_string()).description("loop the current song"),
    ]
}

fn create_play_command() -> CreateCommand<'static> {
    CreateCommand::new(UserCommand::play.to_string())
        .description("play song from youtube")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "query",
                "what to search youtube for",
            )
            .required(true),
        )
}

fn create_play_video_command() -> CreateCommand<'static> {
    CreateCommand::new(UserCommand::play_from_attachment.to_string()).kind(CommandType::Message)
}

fn create_skip_command() -> CreateCommand<'static> {
    CreateCommand::new(UserCommand::skip.to_string())
        .description("skip one or multiple songs")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "number",
                "skip the requested song in the queue",
            )
            .required(false),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "until",
                "skip all the songs before the requested song",
            )
            .required(false),
        )
}

fn create_swap_command() -> CreateCommand<'static> {
    CreateCommand::new(UserCommand::swap_songs.to_string())
        .description("swap the positions of 2 songs in the queue")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "first-track",
                "The first track to swap",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "second-track",
                "The second track to swap",
            )
            .required(true),
        )
}
