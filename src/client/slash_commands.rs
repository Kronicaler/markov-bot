use std::str::FromStr;

use crate::*;
use serenity::{
    client::Context,
    model::interactions::application_command::{
        ApplicationCommand, ApplicationCommandInteraction, ApplicationCommandOptionType,
    },
};
#[allow(unused_imports)]
use std::string::ToString;
use strum_macros::{Display, EnumString};

#[allow(non_camel_case_types)]
#[derive(Display, EnumString)]
pub enum Command {
    ping,
    id,
    blacklistedmarkov,
    blacklistmarkov,
    createtag,
    removetag,
    tags,
    blacklistmefromtags,
    setbotchannel,
    help,
    #[strum(serialize = "blue")]
    testcommand,
    command,
    version
}

pub async fn command_responses(command: &ApplicationCommandInteraction, ctx: Context) {
    let content = match Command::from_str(&command.data.name) {
        Ok(user_command) => match user_command{
            Command::ping => "Hey, I'm alive!".to_owned(),
            Command::id => id_command(command),
            Command::blacklistedmarkov => blacklisted_command(&ctx).await,
            Command::blacklistmarkov => {
                add_or_remove_user_from_markov_blacklist(
                    &command.member.as_ref().unwrap().user,
                    &ctx,
                )
                .await
            }
            Command::testcommand => "here be future tests".to_owned(),
            Command::createtag => set_listener_command(&ctx, command).await,
            Command::removetag => remove_listener_command(&ctx, command).await,
            Command::tags => list_listeners(&ctx).await,
            Command::blacklistmefromtags => {
                blacklist_user_from_listener(&ctx, &command.member.clone().unwrap().user).await
            }
            Command::setbotchannel => set_bot_channel(&ctx, command).await,
            Command::help => "All of my commands are slash commands.\n\n\n\n/ping: Pong!\n\n/id: gives you the user id of the selected user\n\n/blacklistedmarkov: lists out the users the bot will not learn from\n\n/blacklistmarkov: blacklist yourself from the markov chain if you don't want the bot to store your messages and learn from them\n\n/setbotchannel: for admins only, set the channel the bot will talk in, if you don't want users using the bot anywhere else you'll have to do it with roles\n\n/createtag: create a tag that the bot will listen for and then respond to when it is said\n\n/removetag: remove a tag\n\n/tags: list out the current tags\n\n/blacklistmefromtags: blacklist yourself from tags so the bot won't ping you if you trip off a tag".to_owned(),
            Command::command => "command".to_owned(),
            Command::version => "My current version is ".to_owned() + env!("CARGO_PKG_VERSION")
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
        println!("Cannot respond to slash command: {}", why);
    }
}

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
                command.name(Command::blacklistedmarkov).description(
                    "Get the list of blacklisted users from the markov learning program",
                )
            })
            .create_application_command(|command| {
                command.name(Command::blacklistmarkov).description(
                    "Blacklist yourself if you don't want me to save and learn from your messages",
                )
            })
            .create_application_command(|command| {
                command.name(Command::setbotchannel).description(
                    "Set this channel as the channel where the bot will send messages in",
                )
            })
            .create_application_command(|command| {
                command
                    .name(Command::help)
                    .description("Information about the bots commands")
            })
            .create_application_command(|command|{
                command
                .name(Command::version)
                .description("The current version of the bot")
            });
        create_listener_commands(commands)
    })
    .await
    .unwrap();
}

pub async fn create_guild_commands(ctx: &Context) {
    GuildId(724_690_339_054_486_107)
        .set_application_commands(&ctx.http, |commands| {
            commands.create_application_command(|command| {
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
        })
        .await
        .unwrap();
}
