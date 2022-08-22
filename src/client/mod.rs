pub mod file_operations;
pub mod global_data;
pub mod helper_funcs;
pub mod markov;
pub mod slash_commands;
pub mod tags;
pub mod voice;

use file_operations::create_file_if_missing;
use global_data::{init_global_data_for_client, HELP_MESSAGE};
use helper_funcs::leave_unknown_guilds;
use slash_commands::{command_responses, create_global_commands, create_test_commands};
use sqlx::{MySql, Pool};

use self::{
    tags::{blacklist_user, respond_to_tag},
    voice::{change_queue_page, helper_funcs::leave_vc_if_alone},
};
use super::tags::check_for_tag_listeners;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{
        channel::Message,
        gateway::Ready,
        id::UserId,
        prelude::{
            interaction::{Interaction, MessageFlags},
            Guild,
        },
        voice::VoiceState,
    },
    prelude::GatewayIntents,
    Client,
};
use songbird::{
    driver::{
        retry::{ExponentialBackoff, Retry, Strategy},
        DecodeMode,
    },
    Config, SerenityInit,
};
use std::{env, str::FromStr};
use strum_macros::{Display, EnumString};
use tokio::join;

#[derive(Display, EnumString, PartialEq, Eq)]
pub enum ButtonIds {
    BlacklistMeFromTags,
    QueueNext,
    QueuePrevious,
}

struct Handler {
    pool: Pool<MySql>,
}

#[async_trait]
impl EventHandler for Handler {
    /// Is called when the bot connects to discord
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let t1 = create_global_commands(&ctx);
        let t2 = leave_unknown_guilds(&ready, &ctx);

        if cfg!(debug_assertions) {
            let t3 = create_test_commands(&ctx);
            join!(t1, t2, t3);
        } else {
            t1.await;
        }
    }
    // Is called when the bot gets data for a guild
    // if is_new is true then the bot just joined a new guild
    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: bool) {
        if is_new {
            let owner = guild.member(&ctx.http, guild.owner_id).await.unwrap().user;

            println!("Joined guild {} owned by {}", guild.name, owner.tag());

            owner.direct_message(&ctx.http, |msg|
                msg.content("
Hi I'm a general purpose bot. I can play music, chat and have tag functionality. type /help if you want to see all of my commands.\n\n
Due to my chatting functionality i save every message that gets said in the server. These saved messages aren't linked to any usernames so they're anonymized.
The owner of the server can prevent the saving of messages in certain channels (/stop-saving-messages-channel) or in the whole server (/stop-saving-messages-server)
and the users can choose themselves if they don't want their messages saved (/stop-saving-my-messages)")
            ).await.unwrap();
        }
    }

    /// Is called when a user starts an [`Interaction`]
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Ping(_) => todo!(),
            Interaction::ApplicationCommand(command) => {
                command_responses(&command, ctx, &self.pool).await;
            }
            Interaction::MessageComponent(mut component) => {
                let button_id =
                    ButtonIds::from_str(&component.data.custom_id).expect("unexpected button ID");

                match button_id {
                    ButtonIds::BlacklistMeFromTags => {
                        let response = blacklist_user(&ctx, &component.user).await;

                        component
                            .create_interaction_response(&ctx.http, |r| {
                                r.interaction_response_data(|d| {
                                    d.content(response).flags(MessageFlags::EPHEMERAL)
                                })
                            })
                            .await
                            .expect("couldn't create response");
                    }
                    ButtonIds::QueueNext | ButtonIds::QueuePrevious => {
                        change_queue_page(&ctx, &mut component, button_id).await;
                    }
                };
            }
            _ => {}
        };
    }

    /// Is called by the framework whenever a user sends a message in a guild or in the bots DMs
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        markov::add_message_to_chain(&msg, &ctx, &self.pool)
            .await
            .ok();

        let words_in_message = msg
            .content
            .to_lowercase()
            .split(' ')
            .map(ToString::to_string)
            .collect::<Vec<String>>();

        if let Some(response) =
            check_for_tag_listeners(&ctx, &words_in_message, msg.author.id, &self.pool).await
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

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        leave_vc_if_alone(old, &ctx).await;

        if new.channel_id.is_none() && new.user_id == ctx.http.application_id().unwrap() {
            let manager = songbird::get(&ctx).await.unwrap();

            let call_lock = manager.get(new.guild_id.unwrap()).unwrap();
            let mut call = call_lock.lock().await;

            call.queue().stop();
            call.remove_all_global_events();
        }
    }
}

pub async fn start() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a DISCORD_TOKEN in the environment");
    let application_id: UserId = env::var("APPLICATION_ID")
        .expect("Expected an APPLICATION_ID in the environment")
        .parse()
        .expect("Couldn't parse the APPLICATION_ID");

    let songbird_config = Config::default()
        .decode_mode(DecodeMode::Pass)
        .driver_retry(Retry {
            retry_limit: Some(2),
            strategy: Strategy::Backoff(ExponentialBackoff::default()),
        })
        .preallocated_tracks(2);

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::non_privileged();

    let database_url =
        env::var("DATABASE_URL").expect("Expected a DATABASE_URL in the environment");
    let pool = sqlx::MySqlPool::connect(&database_url).await.unwrap();

    let mut client = Client::builder(token, intents)
        .application_id(application_id.0)
        .event_handler(Handler { pool })
        .register_songbird_from_config(songbird_config)
        .await
        .expect("Error creating client");

    init_global_data_for_client(&client)
        .await
        .expect("Couldn't initialize global data");

    client.start().await.expect("Couldn't start the client");
}
