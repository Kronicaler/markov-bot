use crate::*;
use serenity::{
    client::Context,
    model::interactions::{ApplicationCommandInteractionData, Interaction},
};

pub async fn command_responses(
    data: &ApplicationCommandInteractionData,
    ctx: Context,
    interaction: &Interaction,
) {
    let content = match data.name.as_str() {
        "ping" => "Hey, I'm alive!".to_string(),
        "id" => id_command(data),
        "blacklistedmarkov" => blacklisted_command(&ctx).await,
        "blacklistmarkov" => {
            add_or_remove_user_from_markov_blacklist(
                &interaction.clone().member.unwrap().user,
                &ctx,
            )
            .await
        }
        "test-command" => "here be future tests".to_string(),
        "createtag" => set_listener_command(&ctx, data).await,
        "removetag" => remove_listener_command(&ctx, data).await,
        "tags" => list_listeners(&ctx).await,
        "blacklistmefromtags" => {
            blacklist_user_from_listener(&ctx, &interaction.member.clone().unwrap().user).await
        }
        "setbotchannel" => set_bot_channel(&ctx, interaction).await,
        "help" => "All of my commands are slash commands.\n\n\n\n/ping: Pong!\n\n/id: gives you the user id of the selected user\n\n/blacklistedmarkov: lists out the users the bot will not learn from\n\n/blacklistmarkov: blacklist yourself from the markov chain if you don't want the bot to store your messages and learn from them\n\n/setbotchannel: for admins only, set the channel the bot will talk in, if you don't want users using the bot anywhere else you'll have to do it with roles\n\n/createtag: create a tag that the bot will listen for and then respond to when it is said\n\n/removetag: remove a tag\n\n/tags: list out the current tags\n\n/blacklistmefromtags: blacklist yourself from tags so the bot won't ping you if you trip off a tag".to_string(),
        "command" => "command".to_string(),
        _ => "not implemented :(".to_string(),
    };
    if let Err(why) = interaction
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
    ApplicationCommand::create_global_application_commands(&ctx.http, |commands| {
        commands
            .create_application_command(|command| {
                command.name("ping").description("A ping command")
            })
            .create_application_command(|command| {
                command
                    .name("id")
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
                command.name("blacklistedmarkov").description(
                    "Get the list of blacklisted users from the markov learning program",
                )
            })
            .create_application_command(|command| {
                command.name("blacklistmarkov").description(
                    "Blacklist yourself if you don't want me to save and learn from your messages",
                )
            })
            .create_application_command(|command| {
                command.name("setbotchannel").description(
                    "Set this channel as the channel where the bot will send messages in",
                )
            })
            .create_application_command(|command| {
                command.name("help").description("Information about the bots commands")
            });
        create_listener_commands(commands)
    })
    .await
    .unwrap();
}

pub async fn create_guild_commands(ctx: &Context) {
    GuildId(724_690_339_054_486_107)
        .create_application_commands(&ctx.http, |commands| {
            commands.create_application_command(|command| {
                command
                    .name("command")
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
