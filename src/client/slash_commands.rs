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
    client::voice::queue::{command_response::queue, shuffle::shuffle_queue},
    global_data, markov, voice, GuildId,
};
use serenity::{
    builder::{
        CreateApplicationCommand, CreateApplicationCommandOption, CreateInteractionResponse,
        CreateInteractionResponseData,
    },
    client::Context,
    model::application::command::CommandOptionType,
    model::prelude::{
        command::Command, interaction::application_command::ApplicationCommandInteraction,
    },
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
pub async fn command_responses(
    command: &ApplicationCommandInteraction,
    ctx: Context,
    pool: &Pool<MySql>,
) {
    let user = &command.user;

    let full_command_name = get_full_command_name(command);

    info!(
        "user '{}' called command '{}'",
        command.user.name, full_command_name
    );

    match UserCommand::from_str(&full_command_name) {
        Ok(user_command) => match user_command {
            UserCommand::ping => ping_command(ctx, command).await,
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
                .create_interaction_response(
                    ctx.http,
                    CreateInteractionResponse::new().interaction_response_data(
                        CreateInteractionResponseData::new().content(global_data::HELP_MESSAGE),
                    ),
                )
                .instrument(info_span!("Sending message"))
                .await
                .expect("Error creating interaction response"),
            UserCommand::version => command
                .create_interaction_response(
                    ctx.http,
                    CreateInteractionResponse::new().interaction_response_data(
                        CreateInteractionResponseData::new().content(
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
            UserCommand::skip => voice::skip(&ctx, command).await,
            UserCommand::stop => voice::stop(&ctx, command).await,
            UserCommand::playing => voice::playing(&ctx, command).await,
            UserCommand::queue => queue(&ctx, command).await,
            UserCommand::loop_song => voice::loop_song(&ctx, command).await,
            UserCommand::swap_songs => voice::swap(&ctx, command).await,
            UserCommand::queue_shuffle => shuffle_queue(&ctx, command).await,
        },
        Err(why) => {
            error!("Cannot respond to slash command {why}");
        }
    };
}

/// Create the slash commands
pub async fn create_global_commands(ctx: &Context) {
    let mut commands = vec![
        CreateApplicationCommand::new(UserCommand::ping.to_string()).description("A ping command"),
        CreateApplicationCommand::new(UserCommand::id.to_string())
            .description("The user to lookup")
            .add_option(
                CreateApplicationCommandOption::new(
                    CommandOptionType::User,
                    "id",
                    "The user to lookup",
                )
                .required(true),
            ),
        CreateApplicationCommand::new(UserCommand::help.to_string())
            .description("Information about my commands"),
        CreateApplicationCommand::new(UserCommand::version.to_string())
            .description("My current version"),
    ];
    commands.append(&mut create_markov_commands());
    commands.append(&mut create_voice_commands());
    commands.push(create_tag_commands());

    Command::set_global_application_commands(&ctx.http, commands)
        .await
        .expect("Couldn't create global slash commands");
}

/// For testing purposes. Creates commands for a specific guild
pub async fn create_test_commands(ctx: &Context) {
    let testing_guild = std::env::var("TESTING_GUILD_ID")
        .expect("Expected a TESTING_GUILD_ID in the environment")
        .parse()
        .expect("Couldn't parse the TESTING_GUILD_ID");

    let test_commands: Vec<CreateApplicationCommand> = vec![];

    GuildId(testing_guild)
        .set_application_commands(&ctx.http, test_commands)
        .await
        .expect("Couldn't create guild test commands");
}
