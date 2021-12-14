use std::str::FromStr;

use crate::*;
use serenity::{
    client::Context,
    model::interactions::application_command::{
        ApplicationCommand, ApplicationCommandInteraction, ApplicationCommandOptionType,
    },
};
use strum_macros::{Display, EnumString};

/// All the slash commands the bot has implemented
#[allow(non_camel_case_types)]
#[derive(Display, EnumString)]
pub enum Command {
    ping,
    id,
    blacklisteddata,
    stopsavingmymessages,
    continuesavingmymessages,
    createtag,
    removetag,
    tags,
    blacklistmefromtags,
    setbotchannel,
    help,
    #[strum(serialize = "test-command")]
    testcommand,
    command,
    version,
}

/// Check which slash command was triggered, call the appropriate function and return a response to the user
pub async fn command_responses(command: &ApplicationCommandInteraction, ctx: Context) {
    let user = &command.member.as_ref().unwrap().user;

    let content = match Command::from_str(&command.data.name) {
        Ok(user_command) => match user_command {
            Command::ping => "Hey, I'm alive!".to_owned(),
            Command::id => id_command(command),
            Command::blacklisteddata => markov::blacklisted_users(&ctx).await,
            Command::stopsavingmymessages => match markov::add_user_to_blacklist(&user, &ctx).await {
                Ok(_) => format!(
                    "Added {} to data collection blacklist",
                    user.nick_in(&ctx.http, &command.guild_id.unwrap())
                        .await
                        .unwrap_or_else(|| { command.member.as_ref().unwrap().user.name.clone() })
                ),
                Err(_) => "Something went wrong while adding you to the blacklist :(".to_owned(),
            },
            Command::testcommand => test_command(),
            Command::createtag => set_listener_command(&ctx, command).await,
            Command::removetag => remove_listener_command(&ctx, command).await,
            Command::tags => list_listeners(&ctx).await,
            Command::blacklistmefromtags => {
                blacklist_user_from_listener(&ctx, &command.member.clone().unwrap().user).await
            }
            Command::setbotchannel => set_bot_channel(&ctx, command).await,
            Command::help => HELP_MESSAGE.to_owned(),
            Command::command => "command".to_owned(),
            Command::version => "My current version is ".to_owned() + env!("CARGO_PKG_VERSION"),
            Command::continuesavingmymessages => match markov::remove_user_from_blacklist(&user, &ctx)
                .await
            {
                Ok(_) => format!(
                    "removed {} from data collection blacklist",
                    user.nick_in(&ctx.http, &command.guild_id.unwrap())
                        .await
                        .unwrap_or_else(|| { command.member.as_ref().unwrap().user.name.clone() })
                ),
                Err(_) => {
                    "Something went wrong while removing you from the blacklist :(".to_owned()
                }
            },
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
                command.name(Command::setbotchannel).description(
                    "Set this channel as the channel where i will send messages in",
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
        create_listener_commands(commands)
    })
    .await
    .unwrap();
}

/// For testing purposes
pub async fn create_guild_commands(ctx: &Context) {
    let testing_guild = 724_690_339_054_486_107;

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
        .unwrap();
}
