use crate::*;
use druid::Target;
use serenity::{
    async_trait,
    client::{bridge::gateway::ShardManager, Context, EventHandler},
    framework::{
        standard::macros::{group, hook},
        StandardFramework,
    },
    http::Http,
    model::{interactions::Interaction, prelude::*},
    prelude::*,
    Client,
};
use std::{env, sync::Arc};
use strum_macros::{Display, EnumString};
use tokio::join;

struct Handler {}

#[derive(Display, EnumString)]
pub enum ButtonIds {
    BlacklistMeFromTags,
}

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
                    
                    button.create_interaction_response(&ctx.http, |r| {
                        r.interaction_response_data(|d| {
                            d.content(response)
                        })
                    }).await.unwrap();
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
                    "All of my commands are slash commands.\n\n\n\n/ping: Pong!\n\n/id: gives you the user id of the selected user\n\n/blacklistedmarkov: lists out the users the bot will not learn from\n\n/blacklistmarkov: blacklist yourself from the markov chain if you don't want the bot to store your messages and learn from them\n\n/setbotchannel: for admins only, set the channel the bot will talk in, if you don't want users using the bot anywhere else you'll have to do it with roles\n\n/createtag: create a tag that the bot will listen for and then respond to when it is said\n\n/removetag: remove a tag\n\n/tags: list out the current tags\n\n/blacklistmefromtags: blacklist yourself from tags so the bot won't ping you if you trip off a tag\n\n/version: Check the version of the bot",
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

pub async fn start_client(front_channel: FrontChannelStruct) {
    let token = env::var("DISCORD_TOKEN").expect("Expected a DISCORD_TOKEN in the environment");
    let application_id: UserId = env::var("APPLICATION_ID")
        .expect("Expected an APPLICATION_ID in the environment")
        .parse()
        .expect("Couldn't parse the APPLICATION_ID");
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
        init_global_data_for_client(&client, front_channel).await;
    }

    select! {
        _ = listener(client.data.clone(),client.shard_manager.clone()) =>{println!("listener completed first")}
        _ = client.start() => {println!("client completed first")}
    }
}

pub async fn listener(
    data: Arc<serenity::prelude::RwLock<TypeMap>>,
    shard_manager: Arc<Mutex<ShardManager>>,
) {
    loop {
        let front_channel_lock = get_front_channel_lock(&data).await;
        let front_channel = front_channel_lock.read().await;

        if let Ok(_) = front_channel.export_and_quit_receiver.try_recv() {
            {
                let (markov_chain_lock, event_sink_lock) =
                    tokio::join!(get_markov_chain_lock(&data), get_front_channel_lock(&data));
                let (markov_chain, event_sink) =
                    tokio::join!(markov_chain_lock.write(), event_sink_lock.read());

                if let Err(_) = export_to_markov_file(&markov_chain.clone().export()) {
                    send_markov_export_failure(&event_sink).await;
                    continue;
                }

                event_sink
                    .event_sink
                    .submit_command(EXPORTED_MARKOV_CHAIN, ExportStatus::Success, Target::Auto)
                    .unwrap();
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            shard_manager.lock().await.shutdown_all().await;
            return;
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

async fn send_markov_export_failure(
    event_sink: &tokio::sync::RwLockReadGuard<'_, FrontChannelStruct>,
) {
    event_sink
        .event_sink
        .submit_command(EXPORTED_MARKOV_CHAIN, ExportStatus::Failure, Target::Auto)
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    event_sink
        .event_sink
        .submit_command(EXPORTED_MARKOV_CHAIN, ExportStatus::None, Target::Auto)
        .unwrap();
}
