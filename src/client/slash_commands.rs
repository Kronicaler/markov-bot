use super::{
    helper_funcs::user_id_command,
    tags::{
        blacklist_user_from_tags, create_tag, create_tag_commands, list_tags, remove_tag,
        set_tag_response_channel,
    },
};
use crate::*;
use serenity::{
    client::Context,
    model::interactions::{application_command::{
        ApplicationCommand, ApplicationCommandInteraction, ApplicationCommandOptionType,
    }, InteractionResponseType}, builder::CreateEmbed,
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
    #[strum(serialize = "test-command")]
    testcommand,
    command,
    version,

    // =====VOICE=====
    join,
    play,
    skip,
    stop,
    playing,
    queue,
}

pub enum ResponseType{
    Content(String),
    Embed(CreateEmbed)
}

/// Check which slash command was triggered, call the appropriate function and return a response to the user
pub async fn command_responses(command: &ApplicationCommandInteraction, ctx: Context) {
    let user = &command.user;

    let content: ResponseType = match Command::from_str(&command.data.name) {
        Ok(user_command) => match user_command {
            Command::ping => ResponseType::Content("Hey, I'm alive!".to_owned()),
            Command::id => ResponseType::Content(user_id_command(command)),
            Command::blacklisteddata => ResponseType::Content(markov::blacklisted_users(&ctx).await),
            Command::stopsavingmymessages => {
                match markov::add_user_to_blacklist(user, &ctx).await {
                    Ok(_) => ResponseType::Content(format!(
                        "Added {} to data collection blacklist",
                        match command.guild_id {
                            Some(guild_id) => user
                                .nick_in(&ctx.http, guild_id)
                                .await
                                .or_else(|| Some(user.name.clone()))
                                .expect("Should always have Some value"),
                            None => user.name.clone(),
                        }
                    )),
                    Err(_) => {
                        ResponseType::Content("Something went wrong while adding you to the blacklist :(".to_owned())
                    }
                }
            }
            Command::testcommand => ResponseType::Content(test_command()),
            Command::createtag => ResponseType::Content(create_tag(&ctx, command).await),
            Command::removetag => ResponseType::Content(remove_tag(&ctx, command).await),
            Command::tags => ResponseType::Content(list_tags(&ctx).await),
            Command::blacklistmefromtags => ResponseType::Content(blacklist_user_from_tags(&ctx, user).await),
            Command::settagresponsechannel => ResponseType::Content(set_tag_response_channel(&ctx, command).await),
            Command::help => ResponseType::Content(global_data::HELP_MESSAGE.to_owned()),
            Command::command => ResponseType::Content("command".to_owned()),
            Command::version => ResponseType::Content("My current version is ".to_owned() + env!("CARGO_PKG_VERSION")),
            Command::continuesavingmymessages => {
                match markov::remove_user_from_blacklist(user, &ctx).await {
                    Ok(_) => ResponseType::Content(format!(
                        "removed {} from data collection blacklist",
                        match command.guild_id {
                            Some(guild_id) => user
                                .nick_in(&ctx.http, guild_id)
                                .await
                                .or_else(|| Some(user.name.clone()))
                                .expect("Should always have Some value"),
                            None => user.name.clone(),
                        }
                    )),
                    Err(_) => {
                        ResponseType::Content("Something went wrong while removing you from the blacklist :(".to_owned())
                    }
                }
            }

            // ===== VOICE =====
            Command::join => ResponseType::Content(voice::join(&ctx, command).await),
            Command::play => voice::play(&ctx, command).await,
            Command::skip => ResponseType::Content(voice::skip(&ctx, command).await),
            Command::stop => ResponseType::Content(voice::stop(&ctx, command).await),
            Command::playing => ResponseType::Content(voice::playing(&ctx, command).await),
            Command::queue => ResponseType::Content(voice::queue(&ctx, command).await),
        },
        Err(_) => ResponseType::Content("not implemented :(".to_owned()),
    };

    if let Err(why) = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| match content {
                    ResponseType::Content(str) => message.content(str),
                    ResponseType::Embed(e) => message.add_embed(e),
                })
        })
        .await
    {
        eprintln!("Cannot respond to slash command: {why}");
    }
}

fn test_command() -> String {
    "here be tests".to_owned()
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
    let testing_guild = 238633570439004162; // TODO: make into an optional environment variable

    GuildId(testing_guild)
        .set_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(|command| {
                    command
                        .name(Command::command)
                        .description("this is a command")
                        .create_option(|option| {
                            option
                                .name("option")
                                .description("this is an option")
                                .kind(ApplicationCommandOptionType::SubCommand)
                                .create_sub_option(|suboption| {
                                    suboption
                                        .name("suboption")
                                        .description("this is a suboption")
                                        .kind(ApplicationCommandOptionType::Boolean)
                                })
                                .create_sub_option(|suboption| {
                                    suboption
                                        .name("suboption2")
                                        .description("this is a suboption")
                                        .kind(ApplicationCommandOptionType::Boolean)
                                })
                        })
                        .create_option(|option| {
                            option
                                .name("option2")
                                .description("this is an option")
                                .kind(ApplicationCommandOptionType::SubCommand)
                                .create_sub_option(|suboption| {
                                    suboption
                                        .name("suboption3")
                                        .description("this is a suboption")
                                        .kind(ApplicationCommandOptionType::Boolean)
                                })
                        })
                })
                .create_application_command(|command| {
                    command
                        .name(Command::testcommand)
                        .description("test command".to_owned())
                })
                // ===== VOICE =====
                //join voice channel
                .create_application_command(|command| {
                    command
                        .name(Command::join)
                        .description("join voice channel")
                })
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
