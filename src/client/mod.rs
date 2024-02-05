pub mod file_operations;
pub mod global_data;
pub mod helper_funcs;
pub mod markov;
pub mod slash_commands;
pub mod tags;
pub mod voice;

use global_data::{init_global_data_for_client, HELP_MESSAGE};
use slash_commands::{command_responses, create_global_commands, create_test_commands};
use sqlx::{MySql, Pool};
use tracing::{info_span, Instrument};

use self::{
    tags::{blacklist_user, respond_to_tag},
    voice::{
        component_interactions::{
            bring_to_front::bring_to_front, change_queue_page::change_queue_page,
            play_now::play_now, skip::skip_button_press,
        },
        helper_funcs::leave_vc_if_alone,
    },
};
use super::tags::check_for_tag_listeners;
use serenity::{
    async_trait,
    builder::{CreateInteractionResponse, CreateInteractionResponseData, CreateMessage},
    client::{Context, EventHandler},
    model::{
        channel::Message,
        gateway::Ready,
        id::UserId,
        prelude::{interaction::Interaction, Guild, MessageFlags},
        voice::VoiceState,
    },
    prelude::GatewayIntents,
    Client,
};
use songbird::{
    driver::retry::{Retry, Strategy},
    Config, SerenityInit,
};
use std::{env, str::FromStr, time::Duration};
use strum_macros::{Display, EnumString};
use tokio::{join, time::timeout};

#[derive(Display, EnumString, Debug, PartialEq, Eq, Clone, Copy)]
pub enum ComponentIds {
    BlacklistMeFromTags,
    QueueNext,
    QueuePrevious,
    Skip,
    PlayNow,
    PlayNowMenu,
    BringToFront,
    BringToFrontMenu,
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

        if cfg!(debug_assertions) {
            let t3 = create_test_commands(&ctx);
            join!(t1, t3);
        } else {
            t1.await;
        }
    }
    // Is called when the bot gets data for a guild
    // if is_new is true then the bot just joined a new guild
    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: bool) {
        let owner = guild
            .member(&ctx.http, guild.owner_id)
            .await
            .unwrap()
            .user
            .clone();

        if is_new {
            println!(
                "Joined guild {} owned by {} with {} members",
                guild.name,
                owner.tag(),
                guild.member_count
            );

            owner.direct_message(&ctx.http, CreateMessage::new().content("
Hi, I was just invited to your server by an admin. I'm a general purpose bot. I can play music, chat and i also have tag functionality. Type /help if you want to see all of my commands.\n\n
Due to my chatting functionality I save every message that gets said in the server. These saved messages aren't linked to any usernames so they're anonymized.
The admins of the server can prevent the saving of messages in certain channels (/stop-saving-messages-channel) or in the whole server (/stop-saving-messages-server)
and the users can choose themselves if they don't want their messages saved (/stop-saving-my-messages)")
            ).await.unwrap();
        } else {
            println!("Got data for guild {} owned by {}", guild.name, owner.tag());
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
                let button_id = ComponentIds::from_str(&component.data.custom_id)
                    .expect("unexpected button ID");

                match button_id {
                    ComponentIds::BlacklistMeFromTags => {
                        let response = blacklist_user(&component.user, &__self.pool).await;
                        component
                            .create_interaction_response(
                                &ctx.http,
                                CreateInteractionResponse::new().interaction_response_data(
                                    CreateInteractionResponseData::new()
                                        .content(response)
                                        .flags(MessageFlags::EPHEMERAL),
                                ),
                            )
                            .instrument(info_span!("Sending message"))
                            .await
                            .expect("couldn't create response");
                    }
                    ComponentIds::QueueNext | ComponentIds::QueuePrevious => {
                        change_queue_page(&ctx, &mut component, button_id).await;
                    }
                    ComponentIds::Skip => {
                        skip_button_press(&ctx, &component).await;
                    }
                    ComponentIds::PlayNow | ComponentIds::PlayNowMenu => {
                        play_now(&ctx, &component).await;
                    }
                    ComponentIds::BringToFront | ComponentIds::BringToFrontMenu => {
                        bring_to_front(&ctx, &component).await;
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
            .replace('\n', " ")
            .split(' ')
            .map(ToString::to_string)
            .collect::<Vec<String>>();

        if msg.guild_id.is_some() {
            if let Some(response) = check_for_tag_listeners(
                &words_in_message,
                msg.author.id,
                msg.guild_id.unwrap().get(),
                &self.pool,
            )
            .await
            {
                respond_to_tag(&ctx, &msg, &response, &self.pool).await;
                return;
            }
        }

        if msg
            .mentions_me(&ctx.http)
            .await
            .expect("Couldn't read cache")
        {
            async move {
                if words_in_message.contains(&"help".to_owned()) {
                    msg.channel_id
                        .say(&ctx.http, HELP_MESSAGE)
                        .instrument(info_span!("Sending message"))
                        .await
                        .expect("Couldn't send message");
                    return;
                }

                msg.channel_id
                    .say(&ctx.http, markov::generate_sentence(&ctx).await)
                    .instrument(info_span!("Sending message"))
                    .await
                    .expect("Couldn't send message");
            }
            .instrument(info_span!("Mentioned"))
            .await;
        }
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        leave_vc_if_alone(old, &ctx).await;

        if new.channel_id.is_none() && new.user_id == ctx.http.application_id().unwrap().get() {
            let manager = songbird::get(&ctx).await.unwrap();

            let call_lock = manager.get(new.guild_id.unwrap()).unwrap();
            let mut call = timeout(Duration::from_secs(30), call_lock.lock())
                .await
                .unwrap();

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
        .driver_retry(Retry {
            retry_limit: Some(60),
            strategy: Strategy::Every(std::time::Duration::from_secs(10)),
        })
        .preallocated_tracks(2);

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::non_privileged();

    let database_url =
        env::var("DATABASE_URL").expect("Expected a DATABASE_URL in the environment");
    let pool = sqlx::MySqlPool::connect(&database_url).await.unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let mut client = Client::builder(token, intents)
        .application_id(application_id.get())
        .event_handler(Handler { pool })
        .register_songbird_from_config(songbird_config)
        .await
        .expect("Error creating client");

    init_global_data_for_client(&client)
        .await
        .expect("Couldn't initialize global data");

    client.start().await.expect("Couldn't start the client");
}
