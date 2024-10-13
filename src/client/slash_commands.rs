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
        helper_funcs::{download_command, download_from_message_command},
        voice::queue::{command_response::queue, shuffle::shuffle_queue},
    },
    global_data, markov, voice, GuildId,
};
use serenity::{
    all::{
        Command, CommandInteraction, CommandOptionType, CommandType,
        CreateInteractionResponseMessage, EditInteractionResponse, InstallationContext,
        InteractionContext,
    },
    builder::{CreateCommand, CreateCommandOption, CreateInteractionResponse},
    client::Context,
};
use sqlx::{MySql, Pool};
use strum_macros::{Display, EnumProperty, EnumString};
use tracing::{error, info, info_span, Instrument};

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
    #[strum(serialize = "queue-shuffle")]
    queue_shuffle,
    #[strum(serialize = "loop")]
    loop_song,
    #[strum(serialize = "swap-songs")]
    swap_songs,
}

/// Check which slash command was triggered, call the appropriate function and return a response to the user
#[allow(clippy::too_many_lines)]
#[tracing::instrument(name = "Command", skip(ctx, pool))]
pub async fn command_responses(command: &CommandInteraction, ctx: Context, pool: &Pool<MySql>) {
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
                download_from_message_command(ctx, command).await
            }
            UserCommand::id => user_id_command(ctx, command).await,
            UserCommand::stop_saving_my_messages => {
                markov::add_user_to_blacklist(user, &ctx, command, pool).await;
            }
            UserCommand::create_tag => create_tag(&ctx, command, pool).await,
            UserCommand::remove_tag => remove_tag(&ctx, command, pool).await,
            UserCommand::tag_list => list_tags(&ctx, command, pool).await,
            UserCommand::blacklist_me_from_tags => {
                blacklist_user_from_tags_command(&ctx, user, command, pool).await;
            }

            UserCommand::tag_response_channel => {
                set_tag_response_channel(&ctx, command, pool).await;
            }
            UserCommand::help => command
                .create_response(
                    ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(global_data::HELP_MESSAGE),
                    ),
                )
                .instrument(info_span!("Sending message"))
                .await
                .expect("Error creating interaction response"),
            UserCommand::version => command
                .create_response(
                    ctx.http,
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
                markov::remove_user_from_blacklist(user, &ctx, command, pool).await;
            }
            UserCommand::stop_saving_messages_channel => {
                markov::stop_saving_messages_channel(&ctx, command, pool).await;
            }
            UserCommand::stop_saving_messages_server => {
                markov::stop_saving_messages_server(&ctx, command, pool).await;
            }

            // ===== VOICE =====
            UserCommand::play => voice::play(&ctx, command).await,
            UserCommand::play_from_attachment => voice::play_from_attachment(&ctx, command).await,
            UserCommand::skip => voice::skip(&ctx, command).await,
            UserCommand::stop => voice::stop(&ctx, command).await,
            UserCommand::playing => voice::playing(&ctx, command).await,
            UserCommand::queue => queue(&ctx, command).await,
            UserCommand::loop_song => voice::loop_song(&ctx, command).await,
            UserCommand::swap_songs => voice::swap(&ctx, command).await,
            UserCommand::queue_shuffle => {
                command.defer(&ctx.http).await.unwrap();

                let response = shuffle_queue(&ctx, command.guild_id.unwrap()).await;

                command
                    .edit_response(&ctx.http, EditInteractionResponse::new().content(response))
                    .instrument(info_span!("Sending message"))
                    .await
                    .expect("Error creating interaction response");
            }
        },
        Err(why) => {
            error!("Cannot respond to slash command {why}");
        }
    };
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
    commands.push(create_tag_commands());

    Command::set_global_commands(&ctx.http, commands)
        .await
        .expect("Couldn't create global slash commands");
}

fn create_download_commands() -> Vec<CreateCommand> {
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

/// For testing purposes. Creates commands for a specific guild
pub async fn create_test_commands(ctx: &Context) {
    let testing_guild = std::env::var("TESTING_GUILD_ID")
        .expect("Expected a TESTING_GUILD_ID in the environment")
        .parse()
        .expect("Couldn't parse the TESTING_GUILD_ID");

    let test_commands: Vec<CreateCommand> = vec![];

    GuildId::new(testing_guild)
        .set_commands(&ctx.http, test_commands)
        .await
        .expect("Couldn't create guild test commands");
}
