pub mod file_operations;
pub mod global_data;
pub mod helper_funcs;
pub mod markov;
pub mod slash_commands;
pub mod tags;

pub use file_operations::*;
pub use global_data::*;
pub use helper_funcs::*;
pub use slash_commands::*;

use self::tags::respond_to_tag;
use super::tags::{blacklist_user_from_tags, check_for_tag_listeners};
use serenity::{
    async_trait,
    client::{Context, EventHandler},
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
    /// Is called when the bot connects to discord
    async fn ready(&self, ctx: Context, ready: Ready) {
        leave_unknown_guilds(&ready, &ctx).await;

        println!("{} is connected!", ready.user.name);

        let t1 = ctx.set_activity(Activity::watching("https://github.com/TheKroni/markov-bot"));
        let t2 = create_global_commands(&ctx);
        let t3 = create_test_commands(&ctx);

        join!(t1, t2, t3);
    }
    /// Is called when a user starts an [`Interaction`]
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction.kind() {
            InteractionType::Ping => todo!(),
            InteractionType::ApplicationCommand => {
                let command = interaction.application_command().expect(
                    "it's already known that this is an ApplicationCommand and shouldn't break",
                );
                command_responses(&command, ctx).await;
            }
            InteractionType::MessageComponent => {
                let button = interaction.message_component().expect(
                    "it's already known that this is a message component and shouldn't break",
                );

                if button.data.custom_id == ButtonIds::BlacklistMeFromTags.to_string() {
                    let response = blacklist_user_from_tags(&ctx, &button.user).await;

                    button
                        .create_interaction_response(&ctx.http, |r| {
                            r.interaction_response_data(|d| {
                                d.content(response).flags(
                                    InteractionApplicationCommandCallbackDataFlags::EPHEMERAL,
                                )
                            })
                        })
                        .await
                        .expect("couldn't create response");
                }
            }
            _ => {}
        }
    }

    /// Is called by the framework whenever a user sends a message in a guild or in the bots DMs
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        markov::add_message_to_chain(&msg, &ctx).await.ok();

        let words_in_message = msg
            .content
            .to_lowercase()
            .split(' ')
            .map(ToString::to_string)
            .collect::<Vec<String>>();

        if let Some(response) =
            check_for_tag_listeners(&ctx, &words_in_message, msg.author.id).await
        {
            respond_to_tag(&ctx, &msg, &response).await;
            return;
        }

        if msg
            .mentions_me(&ctx.http)
            .await
            .expect("Couldn't read cache")
        {
            if words_in_message.contains(&"help".to_owned()) {
                msg.channel_id
                    .say(&ctx.http, HELP_MESSAGE)
                    .await
                    .expect("Couldn't send message");
                return;
            }

            msg.channel_id
                .say(&ctx.http, markov::generate_sentence(&ctx).await)
                .await
                .expect("Couldn't send message");
        }
    }
}

pub async fn start_client() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a DISCORD_TOKEN in the environment");
    let application_id: UserId = env::var("APPLICATION_ID")
        .expect("Expected an APPLICATION_ID in the environment")
        .parse()
        .expect("Couldn't parse the APPLICATION_ID");

    let mut client = Client::builder(token)
        .application_id(application_id.0)
        .event_handler(Handler {})
        .await
        .expect("Error creating client");

    init_global_data_for_client(&client)
        .await
        .expect("Couldn't initialize global data");

    client.start().await.expect("Couldn't start the client");
}
