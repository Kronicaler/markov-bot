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
        "setlistener" => set_listener_command(&ctx, data).await,
        "removelistener" => remove_listener_command(&ctx, data).await,
        "listeners" => list_listeners(&ctx).await,
        "blacklistlistener" => {
            blacklist_user_from_listener(&ctx, &interaction.member.clone().unwrap().user).await
        }
        "setbotchannel" => set_bot_channel(&ctx, interaction).await,
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
            });
        create_listener_commands(commands)
    })
    .await
    .unwrap();
}
