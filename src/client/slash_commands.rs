use std::str::FromStr;

use crate::*;
use serenity::{
    client::Context,
    model::interactions::application_command::{
        ApplicationCommand, ApplicationCommandInteraction, ApplicationCommandOptionType,
    },
};
use strum_macros::{Display, EnumString};

use super::tags::{
    blacklist_user_from_tags, create_tag, create_tag_commands, list_tags, remove_tag,
    set_tag_response_channel,
};

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
}

/// Check which slash command was triggered, call the appropriate function and return a response to the user
pub async fn command_responses(command: &ApplicationCommandInteraction, ctx: Context) {
    let user = &command.user;

    let content = match Command::from_str(&command.data.name) {
        Ok(user_command) => match user_command {
            Command::ping => "Hey, I'm alive!".to_owned(),
            Command::id => user_id_command(command),
            Command::blacklisteddata => markov::blacklisted_users(&ctx).await,
            Command::stopsavingmymessages => match markov::add_user_to_blacklist(user, &ctx).await
            {
                Ok(_) => format!(
                    "Added {} to data collection blacklist",
                    match command.guild_id {
                        Some(guild_id) => user
                            .nick_in(&ctx.http, guild_id)
                            .await
                            .or_else(|| Some(user.name.clone()))
                            .expect("Should always have Some value"),
                        None => user.name.clone(),
                    }
                ),
                Err(_) => "Something went wrong while adding you to the blacklist :(".to_owned(),
            },
            Command::testcommand => test_command(),
            Command::createtag => create_tag(&ctx, command).await,
            Command::removetag => remove_tag(&ctx, command).await,
            Command::tags => list_tags(&ctx).await,
            Command::blacklistmefromtags => blacklist_user_from_tags(&ctx, user).await,
            Command::settagresponsechannel => set_tag_response_channel(&ctx, command).await,
            Command::help => HELP_MESSAGE.to_owned(),
            Command::command => "command".to_owned(),
            Command::version => "My current version is ".to_owned() + env!("CARGO_PKG_VERSION"),
            Command::continuesavingmymessages => {
                match markov::remove_user_from_blacklist(user, &ctx).await {
                    Ok(_) => format!(
                        "removed {} from data collection blacklist",
                        match command.guild_id {
                            Some(guild_id) => user
                                .nick_in(&ctx.http, guild_id)
                                .await
                                .or_else(|| Some(user.name.clone()))
                                .expect("Should always have Some value"),
                            None => user.name.clone(),
                        }
                    ),
                    Err(_) => {
                        "Something went wrong while removing you from the blacklist :(".to_owned()
                    }
                }
            }
        },
        Err(_) => "not implemented :(".to_owned(),
    };

    if let Err(why) = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content))
        })
        .await
    {
        eprintln!("Cannot respond to slash command: {}", why);
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

/// For testing purposes
/// 
/// TODO: call only when it's run in debug mode 
pub async fn create_guild_commands(ctx: &Context) {
    let testing_guild = 724_690_339_054_486_107; // TODO: make into an optional environment variable

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
        })
        .await
        .expect("Couldn't create guild test commands");
}
