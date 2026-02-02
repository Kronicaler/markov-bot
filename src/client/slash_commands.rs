use std::str::FromStr;

use super::{
    helper_funcs::{get_full_command_name, ping_command, user_id_command},
    markov::commands::create_markov_commands,
    tags::{
        blacklist_user_from_tags_command, commands::create_tag_commands, create_tag, list_tags,
        remove_tag, set_tag_response_channel,
    },
    voice::commands::create_voice_commands,
};
use crate::{
    client::{
        download::{download_command, download_from_message_command},
        memes::{
            commands::create_memes_commands, meme_categories_command, meme_upload_command,
            post_meme_command, upload_meme_command,
        },
        voice::{
            loop_song::loop_song,
            play::play,
            play_from_attachment::play_from_attachment,
            playing::playing,
            queue::{command_response::queue, shuffle::shuffle_queue},
            skip::skip,
            stop::stop,
            swap::swap,
        },
    },
    global_data, markov,
};
use serenity::{
    all::Context,
    all::{
        Command, CommandInteraction, CommandOptionType, CommandType,
        CreateInteractionResponseMessage, EditInteractionResponse, InstallationContext,
        InteractionContext,
    },
    builder::{CreateCommand, CreateCommandOption, CreateInteractionResponse},
};
use sqlx::{Pool, Postgres};
use strum_macros::{Display, EnumProperty, EnumString};
use tracing::{Instrument, error, info, info_span};

/// All the slash commands the bot has implemented
#[allow(non_camel_case_types)]
#[derive(Display, EnumString, EnumProperty)]
pub enum UserCommand {
    ping,
    id,
    #[strum(serialize = "stop-saving-my-messages")]
    stop_saving_my_messages,
    #[strum(serialize = "continue-saving-my-messages")]
    continue_saving_my_messages,
    #[strum(serialize = "stop-saving-messages-channel")]
    stop_saving_messages_channel,
    #[strum(serialize = "stop-saving-messages-server")]
    stop_saving_messages_server,
    help,
    version,
    download,
    #[strum(serialize = "Download from Link")]
    download_from_message_link,

    // =====TAGS=====
    #[strum(props(SubCommand = "create"), serialize = "tag create")]
    create_tag,
    #[strum(props(SubCommand = "remove"), serialize = "tag remove")]
    remove_tag,
    #[strum(props(SubCommand = "list"), serialize = "tag list")]
    tag_list,
    #[strum(
        props(SubCommand = "stop-pinging-me"),
        serialize = "tag stop-pinging-me"
    )]
    blacklist_me_from_tags,
    #[strum(
        props(SubCommand = "response-channel"),
        serialize = "tag response-channel"
    )]
    tag_response_channel,

    // =====VOICE=====
    play,
    #[strum(serialize = "Play Now")]
    play_from_attachment,
    skip,
    stop,
    playing,
    queue,
    #[strum(serialize = "shuffle-queue")]
    queue_shuffle,
    #[strum(serialize = "loop")]
    loop_song,
    #[strum(serialize = "swap-songs")]
    swap_songs,

    // =====MEME=====
    #[strum(serialize = "Upload meme")]
    upload_meme,
    #[strum(props(SubCommand = "post"), serialize = "meme post")]
    meme_post,
    #[strum(props(SubCommand = "categories"), serialize = "meme categories")]
    meme_categories,
    #[strum(props(SubCommand = "upload"), serialize = "meme upload")]
    meme_upload,
}

/// Check which slash command was triggered, call the appropriate function and return a response to the user
#[allow(clippy::too_many_lines)]
#[tracing::instrument(name = "Command", skip(ctx, pool, command))]
pub async fn command_responses(command: &CommandInteraction, ctx: &Context, pool: &Pool<Postgres>) {
    info!(?command);
    let user = &command.user;

    let full_command_name = get_full_command_name(command);

    info!(
        "user '{}' called command '{}'",
        command.user.name, full_command_name
    );

    match UserCommand::from_str(&full_command_name) {
        Ok(user_command) => match user_command {
            UserCommand::ping => ping_command(ctx, command).await,
            UserCommand::download => download_command(ctx, command).await,
            UserCommand::download_from_message_link => {
                download_from_message_command(ctx, command).await;
            }
            UserCommand::id => user_id_command(ctx, command).await,
            UserCommand::stop_saving_my_messages => {
                markov::add_user_to_blacklist(user, ctx, command, pool).await;
            }
            UserCommand::create_tag => create_tag(ctx, command, pool).await,
            UserCommand::remove_tag => remove_tag(ctx, command, pool).await,
            UserCommand::tag_list => list_tags(ctx, command, pool).await,
            UserCommand::blacklist_me_from_tags => {
                blacklist_user_from_tags_command(ctx, user, command, pool).await;
            }
            UserCommand::tag_response_channel => {
                set_tag_response_channel(ctx, command, pool).await;
            }
            UserCommand::help => command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(global_data::HELP_MESSAGE),
                    ),
                )
                .instrument(info_span!("Sending message"))
                .await
                .expect("Error creating interaction response"),
            UserCommand::version => command
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(
                            "My current version is ".to_owned() + env!("CARGO_PKG_VERSION"),
                        ),
                    ),
                )
                .instrument(info_span!("Sending message"))
                .await
                .expect("Error creating interaction response"),
            UserCommand::continue_saving_my_messages => {
                markov::remove_user_from_blacklist(user, ctx, command, pool).await;
            }
            UserCommand::stop_saving_messages_channel => {
                markov::stop_saving_messages_channel(ctx, command, pool).await;
            }
            UserCommand::stop_saving_messages_server => {
                markov::stop_saving_messages_server(ctx, command, pool).await;
            }
            UserCommand::play => play(ctx, command).await,
            UserCommand::play_from_attachment => play_from_attachment(ctx, command).await,
            UserCommand::skip => skip(ctx, command).await.unwrap(),
            UserCommand::stop => stop(ctx, command).await,
            UserCommand::playing => playing(ctx, command).await,
            UserCommand::queue => queue(ctx, command).await,
            UserCommand::loop_song => loop_song(ctx, command).await,
            UserCommand::swap_songs => swap(ctx, command).await,
            UserCommand::queue_shuffle => {
                command.defer(&ctx.http).await.unwrap();

                let response = shuffle_queue(ctx, command.guild_id.unwrap()).await.unwrap();

                command
                    .edit_response(&ctx.http, EditInteractionResponse::new().content(response))
                    .instrument(info_span!("Sending message"))
                    .await
                    .expect("Error creating interaction response");
            }
            UserCommand::upload_meme => upload_meme_command(ctx, command, pool).await.unwrap(),
            UserCommand::meme_post => post_meme_command(ctx, command, pool).await.unwrap(),
            UserCommand::meme_categories => {
                meme_categories_command(ctx, command, pool).await.unwrap()
            }
            UserCommand::meme_upload => meme_upload_command(ctx, command, pool).await.unwrap(),
        },
        Err(why) => {
            error!("Cannot respond to slash command {why:?}");
            return;
        }
    }
}

/// Create the slash commands
pub async fn create_global_commands(ctx: &Context) {
    let mut commands = vec![
        CreateCommand::new(UserCommand::ping.to_string()).description("A ping command"),
        CreateCommand::new(UserCommand::id.to_string())
            .description("The user to lookup")
            .add_option(
                CreateCommandOption::new(CommandOptionType::User, "id", "The user to lookup")
                    .required(true),
            ),
        CreateCommand::new(UserCommand::help.to_string())
            .description("Information about my commands"),
        CreateCommand::new(UserCommand::version.to_string()).description("My current version"),
    ];
    commands.append(&mut create_download_commands());
    commands.append(&mut create_markov_commands());
    commands.append(&mut create_voice_commands());
    commands.append(&mut create_memes_commands());
    commands.push(create_tag_commands());

    Command::set_global_commands(&ctx.http, &commands)
        .await
        .expect("Couldn't create global slash commands");
}

fn create_download_commands() -> Vec<CreateCommand<'static>> {
    vec![
        CreateCommand::new(UserCommand::download.to_string())
            .description("download a video or audio file from a url")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "url",
                    "The url to download from",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Boolean,
                    "ephemeral",
                    "Whether the command will only be visible to you. Default: True",
                ),
            )
            .add_integration_type(InstallationContext::User)
            .add_integration_type(InstallationContext::Guild)
            .add_context(InteractionContext::Guild)
            .add_context(InteractionContext::PrivateChannel),
        CreateCommand::new(UserCommand::download_from_message_link.to_string())
            .add_integration_type(InstallationContext::User)
            .add_integration_type(InstallationContext::Guild)
            .add_context(InteractionContext::Guild)
            .add_context(InteractionContext::PrivateChannel)
            .kind(CommandType::Message),
    ]
}
