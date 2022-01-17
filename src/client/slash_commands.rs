use super::{
    helper_funcs::{ping_command, user_id_command},
    tags::{
        blacklist_user_from_tags_command, create_tag, create_tag_commands, list_tags, remove_tag,
        set_tag_response_channel,
    },
};
use crate::*;
use serenity::{
    client::Context,
    model::interactions::application_command::{
        ApplicationCommand, ApplicationCommandInteraction, ApplicationCommandOptionType,
    },
};
use std::str::FromStr;
use strum_macros::{Display, EnumString};

/// All the slash commands the bot has implemented
#[allow(non_camel_case_types)]
#[derive(Display, EnumString)]
pub enum Command {
    ping,
    id,
    #[strum(serialize = "blacklisted-data")]
    blacklisteddata,
    #[strum(serialize = "stop-saving-my-messages")]
    stopsavingmymessages,
    #[strum(serialize = "continue-saving-my-messages")]
    continuesavingmymessages,
    #[strum(serialize = "create-tag")]
    createtag,
    #[strum(serialize = "remove-tag")]
    removetag,
    tags,
    #[strum(serialize = "blacklist-me-from-tags")]
    blacklistmefromtags,
    #[strum(serialize = "set-tag-response-channel")]
    settagresponsechannel,
    help,
    version,

    // =====VOICE=====
    play,
    skip,
    stop,
    playing,
    queue,
}

/// Check which slash command was triggered, call the appropriate function and return a response to the user
pub async fn command_responses(command: &ApplicationCommandInteraction, ctx: Context) {
    let user = &command.user;

    match Command::from_str(&command.data.name) {
        Ok(user_command) => match user_command {
            Command::ping => ping_command(ctx, command).await,
            Command::id => user_id_command(ctx, command).await,
            Command::blacklisteddata => markov::blacklisted_users(ctx, command).await,
            Command::stopsavingmymessages => {
                markov::add_user_to_blacklist(user, &ctx, command).await
            }
            Command::createtag => create_tag(&ctx, command).await,
            Command::removetag => remove_tag(&ctx, command).await,
            Command::tags => list_tags(&ctx, command).await,
            Command::blacklistmefromtags => {
                blacklist_user_from_tags_command(&ctx, user, command).await
            }
            Command::settagresponsechannel => set_tag_response_channel(&ctx, command).await,
            Command::help => command
                .create_interaction_response(ctx.http, |r| {
                    r.interaction_response_data(|d| d.content(global_data::HELP_MESSAGE))
                })
                .await
                .expect("Error creating interaction response"),
            Command::version => command
                .create_interaction_response(ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content("My current version is ".to_owned() + env!("CARGO_PKG_VERSION"))
                    })
                })
                .await
                .expect("Error creating interaction response"),
            Command::continuesavingmymessages => {
                markov::remove_user_from_blacklist(user, &ctx, command).await
            }

            // ===== VOICE =====
            Command::play => voice::play(&ctx, command).await,
            Command::skip => voice::skip(&ctx, command).await,
            Command::stop => voice::stop(&ctx, command).await,
            Command::playing => voice::playing(&ctx, command).await,
            Command::queue => voice::queue(&ctx, command).await,
        },
        Err(why) => {
            eprintln!("Cannot respond to slash command {why}");
        }
    };
}

/// Create the slash commands
pub async fn create_global_commands(ctx: &Context) {
    ApplicationCommand::set_global_application_commands(&ctx.http, |commands| {
        commands
            .create_application_command(|command| {
                command.name(Command::ping).description("A ping command")
            })
            .create_application_command(|command| {
                command
                    .name(Command::id)
                    .description("Get a user id")
                    .create_option(|option| {
                        option
                            .name("id")
                            .description("The user to lookup")
                            .kind(ApplicationCommandOptionType::User)
                            .required(true)
                    })
            })
            .create_application_command(|command| {
                command.name(Command::blacklisteddata).description(
                    "Get the list of users who's messages aren't being saved",
                )
            })
            .create_application_command(|command| {
                command.name(Command::stopsavingmymessages).description(
                    "Blacklist yourself if you don't want me to save and learn from your messages",
                )
            })
            .create_application_command(|command| {
                command.name(Command::continuesavingmymessages).description(
                    "Remove yourself from the blacklist if you want me to save and learn from your messages",
                )
            })
            .create_application_command(|command| {
                command.name(Command::settagresponsechannel).description(
                    "Set this channel as the channel where i will reply to tags",
                )
            })
            .create_application_command(|command| {
                command
                    .name(Command::help)
                    .description("Information about my commands")
            })
            .create_application_command(|command| {
                command
                    .name(Command::version)
                    .description("My current version")
            });
        create_tag_commands(commands)
    })
    .await
    .expect("Couldn't create global slash commands");
}

/// For testing purposes. Creates commands for a specific guild
///
/// TODO: call only when it's run in debug mode
pub async fn create_test_commands(ctx: &Context) {
    let testing_guild = 248167504910745600; // TODO: make into an optional environment variable

    GuildId(testing_guild)
        .set_application_commands(&ctx.http, |commands| {
            commands
                // ===== VOICE =====
                //play from youtube
                .create_application_command(|command| {
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
        })
        .await
        .expect("Couldn't create guild test commands");
}
