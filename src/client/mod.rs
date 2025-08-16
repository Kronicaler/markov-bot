pub mod file_operations;
pub mod global_data;
pub mod helper_funcs;
pub mod markov;
pub mod memes;
pub mod slash_commands;
pub mod tags;
pub mod voice;

use global_data::{init_global_data_for_client, HELP_MESSAGE};
use itertools::Itertools;
use regex::Regex;
use slash_commands::{command_responses, create_global_commands};
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
        queue::shuffle::shuffle_queue,
    },
};
use super::tags::check_for_tag_listeners;
use serenity::{
    all::{CreateInteractionResponseMessage, Guild, Interaction, InteractionResponseFlags},
    async_trait,
    builder::{CreateInteractionResponse, CreateMessage},
    client::{Context, EventHandler},
    model::{channel::Message, gateway::Ready, voice::VoiceState},
    prelude::GatewayIntents,
    Client,
};
use songbird::{
    driver::retry::{Retry, Strategy},
    Config, SerenityInit,
};
use std::{env, str::FromStr, time::Duration};
use strum_macros::{Display, EnumString};
use tokio::{select, time::timeout};

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
    QueueStart,
    QueueEnd,
    Shuffle,
}

struct Handler {
    pool: Pool<MySql>,
}

#[async_trait]
impl EventHandler for Handler {
    /// Is called when the bot connects to discord
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        create_global_commands(&ctx).await;
    }
    // Is called when the bot gets data for a guild
    // if is_new is true then the bot just joined a new guild
    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: std::option::Option<bool>) {
        let owner = guild
            .member(&ctx.http, guild.owner_id)
            .await
            .unwrap()
            .user
            .clone();

        if is_new.unwrap_or(false) {
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
            println!(
                "Got data for guild {} owned by {} with {} members",
                guild.name,
                owner.tag(),
                guild.member_count
            );
        }
    }

    /// Is called when a user starts an [`Interaction`]
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Ping(_) => todo!(),
            Interaction::Command(command) => {
                command_responses(&command, ctx, &self.pool).await;
            }
            Interaction::Component(mut component) => {
                let button_id = ComponentIds::from_str(&component.data.custom_id)
                    .expect("unexpected button ID");

                match button_id {
                    ComponentIds::BlacklistMeFromTags => {
                        let response = blacklist_user(&component.user, &__self.pool).await;
                        component
                            .create_response(
                                &ctx.http,
                                CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new()
                                        .content(response)
                                        .flags(InteractionResponseFlags::EPHEMERAL),
                                ),
                            )
                            .instrument(info_span!("Sending message"))
                            .await
                            .expect("couldn't create response");
                    }
                    ComponentIds::QueueNext
                    | ComponentIds::QueuePrevious
                    | ComponentIds::QueueStart
                    | ComponentIds::QueueEnd => {
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
                    ComponentIds::Shuffle => {
                        component.defer(&ctx.http).await.unwrap();
                        shuffle_queue(&ctx, component.guild_id.unwrap()).await;
                    }
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

        if msg
            .mentions_me(&ctx.http)
            .await
            .expect("Couldn't read cache")
        {
            async {
                if words_in_message.contains(&"help".to_owned()) {
                    msg.channel_id
                        .say(&ctx.http, HELP_MESSAGE)
                        .instrument(info_span!("Sending message"))
                        .await
                        .expect("Couldn't send message");
                    return;
                }

                if words_in_message.len() > 1 {
                    let user_regex = Regex::new(r"<@!?(\d+)>").expect("Invalid regular expression");

                    let sanitized_message = words_in_message
                        .iter()
                        .filter(|w| !user_regex.is_match(w))
                        .join(" ");

                    msg.channel_id
                        .say(
                            &ctx.http,
                            markov::generate_sentence(&ctx, Some(&sanitized_message)).await,
                        )
                        .instrument(info_span!("Sending message"))
                        .await
                        .expect("Couldn't send message");
                } else {
                    msg.channel_id
                        .say(&ctx.http, markov::generate_sentence(&ctx, None).await)
                        .instrument(info_span!("Sending message"))
                        .await
                        .expect("Couldn't send message");
                }
            }
            .instrument(info_span!("Mentioned"))
            .await;
            return;
        }

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
        .event_handler(Handler { pool })
        .register_songbird_from_config(songbird_config)
        .await
        .expect("Error creating client");

    init_global_data_for_client(&client)
        .await
        .expect("Couldn't initialize global data");

    let termination_signal = wait_for_signal();
    let client = client.start();
    select! {
        () = termination_signal=>{}
        result = client => {result.unwrap();}
    }
}

/// Waits for a signal that requests a graceful shutdown, like SIGTERM or SIGINT.
#[cfg(unix)]
async fn wait_for_signal_impl() {
    use tokio::signal::unix::{signal, SignalKind};

    // Infos here:
    // https://www.gnu.org/software/libc/manual/html_node/Termination-Signals.html
    let mut signal_terminate = signal(SignalKind::terminate()).unwrap();
    let mut signal_interrupt = signal(SignalKind::interrupt()).unwrap();

    tokio::select! {
        _ = signal_terminate.recv() => tracing::debug!("Received SIGTERM."),
        _ = signal_interrupt.recv() => tracing::debug!("Received SIGINT."),
    };
}

#[cfg(windows)]
async fn wait_for_signal_impl() {
    use tokio::signal::windows;

    let mut signal_c = windows::ctrl_c().unwrap();
    let mut signal_break = windows::ctrl_break().unwrap();
    let mut signal_close = windows::ctrl_close().unwrap();
    let mut signal_shutdown = windows::ctrl_shutdown().unwrap();

    tokio::select! {
        _ = signal_c.recv() => tracing::debug!("Received CTRL_C."),
        _ = signal_break.recv() => tracing::debug!("Received CTRL_BREAK."),
        _ = signal_close.recv() => tracing::debug!("Received CTRL_CLOSE."),
        _ = signal_shutdown.recv() => tracing::debug!("Received CTRL_SHUTDOWN."),
    };
}

/// Registers signal handlers and waits for a signal that
/// indicates a shutdown request.
pub(crate) async fn wait_for_signal() {
    wait_for_signal_impl().await;
}
