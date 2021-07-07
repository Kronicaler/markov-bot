#![windows_subsystem = "windows"]
mod commands;
mod global_data;
mod listener_response;
mod markov_chain_funcs;
mod unit_tests;

use commands::example::*;
use global_data::*;
use listener_response::*;
use markov_chain_funcs::*;
use markov_strings::{self, InputData, Markov};
use regex::Captures;
use serenity::{async_trait, framework::{
        standard::macros::{group, hook},
        StandardFramework,
    }, http::Http, model::{channel::{GuildChannel, Message}, gateway::Ready, guild::Guild, id::{GuildId, UserId}, interactions::*, prelude::{Activity, User}}, prelude::*};
use std::{collections::HashSet, env, fs::{self, OpenOptions}, io::Write, path::Path, sync::Arc};



const KRONI_ID: u64 = 594772815283093524;

struct Handler {}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if interaction.kind == InteractionType::ApplicationCommand {
            if let Some(data) = interaction.data.as_ref() {
                match data {
                    InteractionData::ApplicationCommand(data) => {
                        command_responses(data, ctx, &interaction).await;
                    }
                    _ => {}
                }
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        ctx.set_activity(Activity::watching("https://github.com/TheKroni/doki-bot"))
            .await;

        create_global_commands(&ctx).await;

        GuildId(724690339054486107)
            .create_application_commands(ctx.http, |commands| commands)
            .await
            .unwrap();
    }
}

async fn command_responses(
    data: &ApplicationCommandInteractionData,
    ctx: Context,
    interaction: &Interaction,
) {
    let content = match data.name.as_str() {
        "ping" => "Hey, I'm alive!".to_string(),
        "id" => id_command(data),
        "blacklistedmarkov" => blacklisted_command(&ctx).await,
        "blacklistmarkov" => {
            add_or_remove_user_from_blacklist(&interaction.clone().member.unwrap().user, &ctx).await
        }
        "test-command" => "here be future tests".to_string(),
        "setlistener" => set_listener_command(&ctx, data).await,
        "removelistener" => remove_listener_command(&ctx, data).await,
        "listeners" => list_listeners(&ctx).await,
        "blacklistlistener" => {
            blacklist_user_from_listener(&ctx, &interaction.member.clone().unwrap().user).await
        }
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

async fn create_global_commands(ctx: &Context) {
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

fn id_command(data: &ApplicationCommandInteractionData) -> String {
    let options = data
        .options
        .get(0)
        .expect("Expected user option")
        .resolved
        .as_ref()
        .expect("Expected user object");
    if let ApplicationCommandInteractionDataOptionValue::User(user, _member) = options {
        format!("{}'s id is {}", user, user.id)
    } else {
        "Please provide a valid user".to_string()
    }
}

fn get_first_mentioned_user(msg: &Message) -> Option<&User> {
    for user in &msg.mentions {
        if user.bot {
            continue;
        }
        return Option::Some(user);
    }
    return None;
}

///checks if a file exists and if it doesn't it initializes it
///otherwise it just returns the path back
fn create_file_if_missing<'a>(path: &'a str, contents: &str) -> &'a str {
    if !Path::new(path).exists() {
        fs::write(path, contents).unwrap();
    }
    return path;
}

async fn send_message_to_first_available_channel(ctx: &Context, guild:&Guild, message:&str){
    let channels: Vec<GuildChannel> = guild.channels.iter().map(|(_,channel)|{channel.clone()}).collect();
    for channel in channels {
        match channel.id.say(&ctx.http, message.to_string()).await {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}

#[group]
#[commands(ping)]
struct General;

#[hook]
async fn normal_message(ctx: &Context, msg: &Message) {
    should_add_message_to_markov_file(&msg, &ctx).await;
    {
        let listener_response_lock = get_listener_response_lock(ctx).await;
        let listener_response = listener_response_lock.read().await;
        let listener_blacklisted_users_lock = get_listener_blacklisted_users_lock(ctx).await;
        let listener_blacklisted_users = listener_blacklisted_users_lock.read().await;
        for (listener, response) in listener_response.iter() {
            if msg.content.to_lowercase().contains(listener)
                && !listener_blacklisted_users.contains(&msg.author.id.0)
            {
                send_message_to_first_available_channel(ctx, &msg.guild(&ctx.cache).await.unwrap(), response).await;
                return;
            }
        }
    }

    if msg.mentions_me(&ctx.http).await.unwrap() && !msg.author.bot {
        let words_in_message = msg
            .content
            .to_lowercase()
            .split(' ')
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

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

        if msg.author.id == KRONI_ID && msg.content.to_lowercase().contains("blacklist user") {
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

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let application_id: UserId = env::var("APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .unwrap();

    let http = Http::new_with_token(&token);

    // We will fetch your bot's owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create the framework
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
        init_global_data_for_client(&client).await;
    }

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
