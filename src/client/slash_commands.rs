use std::str::FromStr;

use super::{
    helper_funcs::{get_full_command_name, ping_command, user_id_command},
    markov::commands::{create_markov_commands, MarkovCommandBuilder},
    tags::{
        blacklist_user_from_tags_command, commands::TagCommandBuilder, create_tag, list,
        remove_tag, set_tag_response_channel,
    },
    voice::commands::VoiceCommandBuilder,
};
use crate::{global_data, markov, voice, GuildId};
use serenity::{
    builder::{CreateApplicationCommand, CreateApplicationCommandOption},
    client::Context,
    model::application::command::CommandOptionType,
    model::prelude::{
        command::Command, interaction::application_command::ApplicationCommandInteraction,
    },
};
use sqlx::{MySql, Pool};
use strum_macros::{Display, EnumProperty, EnumString};

/// All the slash commands the bot has implemented
#[allow(non_camel_case_types)]
#[derive(Display, EnumString, EnumProperty)]
pub enum UserCommand {
    ping,
    id,
    #[strum(serialize = "stop-saving-my-messages")]
    stopsavingmymessages,
    #[strum(serialize = "continue-saving-my-messages")]
    continuesavingmymessages,
    #[strum(serialize = "stop-saving-messages-channel")]
    stopsavingmessageschannel,
    #[strum(serialize = "stop-saving-messages-server")]
    stopsavingmessagesserver,
    help,
    version,

    // =====TAGS=====
    #[strum(props(SubCommand = "create"), serialize = "tag create")]
    createtag,
    #[strum(props(SubCommand = "remove"), serialize = "tag remove")]
    removetag,
    #[strum(props(SubCommand = "list"), serialize = "tag list")]
    taglist,
    #[strum(
        props(SubCommand = "stop-pinging-me"),
        serialize = "tag stop-pinging-me"
    )]
    blacklistmefromtags,
    #[strum(
        props(SubCommand = "response-channel"),
        serialize = "tag response-channel"
    )]
    tagresponsechannel,

    // =====VOICE=====
    play,
    skip,
    stop,
    playing,
    queue,
    #[strum(serialize = "loop")]
    loop_song,
    #[strum(serialize = "swap-songs")]
    swap_songs,
}

/// Check which slash command was triggered, call the appropriate function and return a response to the user
#[allow(clippy::too_many_lines)]
pub async fn command_responses(
    command: &ApplicationCommandInteraction,
    ctx: Context,
    pool: &Pool<MySql>,
) {
    let user = &command.user;

    let full_command_name = get_full_command_name(command);

    match UserCommand::from_str(&full_command_name) {
        Ok(user_command) => match user_command {
            UserCommand::ping => ping_command(ctx, command).await,
            UserCommand::id => user_id_command(ctx, command).await,
            UserCommand::stopsavingmymessages => {
                markov::add_user_to_blacklist(user, &ctx, command, pool).await;
            }
            UserCommand::createtag => create_tag(&ctx, command, pool).await,
            UserCommand::removetag => remove_tag(&ctx, command, pool).await,
            UserCommand::taglist => list(&ctx, command, pool).await,
            UserCommand::blacklistmefromtags => {
                blacklist_user_from_tags_command(&ctx, user, command, pool).await;
            }

            UserCommand::tagresponsechannel => set_tag_response_channel(&ctx, command, pool).await,
            UserCommand::help => command
                .create_interaction_response(ctx.http, |r| {
                    r.interaction_response_data(|d| d.content(global_data::HELP_MESSAGE))
                })
                .await
                .expect("Error creating interaction response"),
            UserCommand::version => command
                .create_interaction_response(ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content("My current version is ".to_owned() + env!("CARGO_PKG_VERSION"))
                    })
                })
                .await
                .expect("Error creating interaction response"),
            UserCommand::continuesavingmymessages => {
                markov::remove_user_from_blacklist(user, &ctx, command, pool).await;
            }
            UserCommand::stopsavingmessageschannel => {
                markov::stop_saving_messages_channel(&ctx, command, pool).await;
            }
            UserCommand::stopsavingmessagesserver => {
                markov::stop_saving_messages_server(&ctx, command, pool).await;
            }

            // ===== VOICE =====
            UserCommand::play => voice::play(&ctx, command).await,
            UserCommand::skip => voice::skip(&ctx, command).await,
            UserCommand::stop => voice::stop(&ctx, command).await,
            UserCommand::playing => voice::playing(&ctx, command).await,
            UserCommand::queue => voice::queue(&ctx, command).await,
            UserCommand::loop_song => voice::loop_song(&ctx, command).await,
            UserCommand::swap_songs => voice::swap(&ctx, command).await,
        },
        Err(why) => {
            eprintln!("Cannot respond to slash command {why}");
        }
    };
}

/// Create the slash commands
pub async fn create_global_commands(ctx: &Context) {
    let commands = vec![
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
    ]
    .append(&mut create_markov_commands());

    Command::set_global_application_commands(&ctx.http, |commands| {
        commands
            .create_voice_commands()
            .create_tag_commands()
    })
    .await
    .expect("Couldn't create global slash commands");
}

/// For testing purposes. Creates commands for a specific guild
pub async fn create_test_commands(ctx: &Context) {
    let testing_guild = std::env::var("TESTING_GUILD_ID")
        .expect("Expected a TESTING_GUILD_ID in the environment")
        .parse()
        .expect("Couldn't parse the TESTING_GUILD_ID");

    GuildId(testing_guild)
        .set_application_commands(&ctx.http, |commands| commands)
        .await
        .expect("Couldn't create guild test commands");
}
