use crate::*;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    framework::{
        standard::macros::{group, hook},
        StandardFramework,
    },
    http::Http,
    model::{interactions::Interaction, prelude::*},
    Client,
};
use std::env;
use strum_macros::{Display, EnumString};
use tokio::join;

#[derive(Display, EnumString)]
pub enum ButtonIds {
    BlacklistMeFromTags,
}

struct Handler {}

#[async_trait]
impl EventHandler for Handler {
    /// Is called when the bot connects to discord.
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let t1 = ctx.set_activity(Activity::watching("https://github.com/TheKroni/markov-bot"));
        let t2 = create_global_commands(&ctx);
        let t3 = create_guild_commands(&ctx);

        join!(t1, t2, t3);
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction.kind() {
            InteractionType::Ping => todo!(),
            InteractionType::ApplicationCommand => {
                let command = interaction.application_command().unwrap();
                command_responses(&command, ctx).await;
            }
            InteractionType::MessageComponent => {
                let button = interaction.message_component().unwrap();

                if button.data.custom_id == ButtonIds::BlacklistMeFromTags.to_string() {
                    let response = blacklist_user_from_listener(&ctx, &button.user).await;

                    button
                        .create_interaction_response(&ctx.http, |r| {
                            r.interaction_response_data(|d| {
                                d.content(response).flags(
                                    InteractionApplicationCommandCallbackDataFlags::EPHEMERAL,
                                )
                            })
                        })
                        .await
                        .unwrap();
                }
            }
            InteractionType::Unknown => todo!(),
            _ => {}
        }
    }
}

#[group]
#[commands(ping)]
struct General;

/// Is called by the framework whenever a user sends a message in a server or in the bots DMs
#[hook]
async fn normal_message(ctx: &Context, msg: &Message) {
    should_add_message_to_markov_file(&msg, &ctx).await;
    let words_in_message = msg
        .content
        .to_lowercase()
        .split(' ')
        .map(ToString::to_string)
        .collect::<Vec<String>>();

    if let Some(response) = check_for_listened_words(ctx, &words_in_message, msg.author.id).await {
        send_message_to_first_available_channel(ctx, msg, &response).await;
        return;
    }

    if msg.mentions_me(&ctx.http).await.unwrap() && !msg.author.bot {
        if words_in_message.contains(&"help".to_owned()) {
            msg.channel_id
                .say(
                    &ctx.http,
                    "All of my commands are slash commands.\n\n
                    /ping: Pong!\n
                    /id: gives you the user id of the selected user\n
                    /blacklistedmarkov: lists out the users the bot will not learn from\n
                    /blacklistmarkov: blacklist yourself from the markov chain if you don't want the bot to store your messages and learn from them\n
                    /setbotchannel: for admins only, set the channel the bot will talk in, if you don't want users using the bot anywhere else you'll have to do it with roles\n
                    /createtag: create a tag that the bot will listen for and then respond to when it is said\n
                    /removetag: remove a tag\n
                    /tags: list out the current tags\n
                    /blacklistmefromtags: blacklist yourself from tags so the bot won't ping you if you trip off a tag\n
                    /version: Check the version of the bot",
                )
                .await
                .unwrap();
            return;
        }

        if msg.author.id == OWNER_ID
            && msg.content.to_lowercase().contains("blacklist user")
            && msg.content.to_lowercase().contains("markov")
        {
            let message = blacklist_user_command(&msg, &ctx).await;
            msg.channel_id.say(&ctx.http, message).await.unwrap();
            return;
        }

        send_markov_text(ctx, msg).await;
    }
}

pub async fn start_client() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a DISCORD_TOKEN in the environment");
    let application_id: UserId = env::var("APPLICATION_ID")
        .expect("Expected an APPLICATION_ID in the environment")
        .parse()
        .expect("Couldn't parse the APPLICATION_ID");

    let http = Http::new_with_token(&token);

    let owners = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            owners
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).on_mention(Some(application_id)))
        .group(&GENERAL_GROUP)
        .prefix_only(normal_message)
        .normal_message(normal_message);

    let mut client = Client::builder(token)
        .application_id(application_id.0)
        .framework(framework)
        .event_handler(Handler {})
        .await
        .expect("Error creating client");

    init_global_data_for_client(&client).await.unwrap();

    client.start().await.unwrap();
}
