//#![windows_subsystem = "windows"]
#![deny(warnings, missing_docs, clippy::all)]

//! A discord bot written in rust for fun

mod commands;
mod file_operations;
mod front;
mod global_data;
mod helper_funcs;
mod listener_response;
mod markov_chain_funcs;
mod slash_commands;
mod system_tray;
mod unit_tests;

use commands::example::*;
use druid::ExtEventSink;
use file_operations::*;
use front::*;
use global_data::*;
use helper_funcs::*;
use listener_response::*;
use markov_chain_funcs::*;
use markov_strings::Markov;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    framework::{
        standard::macros::{group, hook},
        StandardFramework,
    },
    futures::join,
    http::Http,
    model::{
        channel::Message,
        gateway::Ready,
        id::{GuildId, UserId},
        interactions::*,
        prelude::Activity,
    },
    Client,
};
use slash_commands::*;
use system_tray::*;
use tokio::sync::mpsc::{self, Receiver, Sender};

use std::{collections::HashSet, env, fs, panic};

const KRONI_ID: u64 = 594_772_815_283_093_524;

struct Handler {}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let t1 = ctx.set_activity(Activity::watching("https://github.com/TheKroni/doki-bot"));
        let t2 = create_global_commands(&ctx);
        let t3 = create_guild_commands(&ctx);

        join!(t1, t2, t3);
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if interaction.kind == InteractionType::ApplicationCommand {
            if let Some(InteractionData::ApplicationCommand(data)) = interaction.data.as_ref() {
                command_responses(data, ctx, &interaction).await;
            }
        }
    }
}

#[group]
#[commands(ping)]
struct General;

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
        if words_in_message.contains(&"stfu".to_string())
            || msg.content.to_lowercase().contains("shut up")
            || msg.content.to_lowercase().contains("shut the fuck up")
            || words_in_message.contains(&"kys".to_string())
            || words_in_message.contains(&"die".to_string())
            || msg.content.to_lowercase().contains("kill yourself")
            || msg.content.to_lowercase().contains("fuck you")
            || msg.content.to_lowercase().contains("fuck u")
            || msg.content.to_lowercase().contains("fuck off")
            || msg.content.to_lowercase().contains("suck my")
        {
            let troglodyte = "Next time you *think* of replying with a failed attempt at sarcasm, try to take the half-an-hour or so your troglodyte brain requires to formulate a coherent thought and decide if you ACTUALLY have a point or if you're just mashing your bumbling ham-hands across the keyboard in the same an invertebrate would as though it were being electrified for some laboratory experiment; Not that there's a marked difference between the two outcomes, as any attempt at communication on your part will invariably arise from mere random firings of your sputtering, weak neurons that ends up indistinguishable either way.";
            msg.reply_mention(&ctx.http, troglodyte)
                .await
                .expect("well fuck");
            return;
        }

        if words_in_message.contains(&"help".to_string()) {
            msg.channel_id
                .say(
                    &ctx.http,
                    "all my commands are prefixed by pinging me\nping : Pong!",
                )
                .await
                .unwrap();
            return;
        }

        if msg.author.id == KRONI_ID
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

#[tokio::main]
async fn main() {
    fs::create_dir("data/markov data").ok();
    dotenv::dotenv().expect("Failed to load .env file");

    let (tx, mut rx): (Sender<ExtEventSink>, Receiver<ExtEventSink>) = mpsc::channel(1);

    tokio::task::spawn_blocking(move || start_gui(tx));

    let event_sink = rx.recv().await.unwrap();

    tokio::select! {
        _ = tokio::task::spawn_blocking(create_tray_icon) =>{},
        _ = start_client(event_sink) =>{},
    }
}

async fn start_client(event_sink: ExtEventSink) {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let application_id: UserId = env::var("APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .unwrap();
    let http = Http::new_with_token(&token);
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
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
        .expect("Err creating client");
    {
        init_global_data_for_client(&client, event_sink).await;
    }
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
