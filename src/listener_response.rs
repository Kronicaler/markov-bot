use std::{
    collections::{HashMap, HashSet},
    fs,
    sync::Arc,
};

use serenity::{
    client::Context,
    model::{
        interactions::{
            ApplicationCommandInteractionData, ApplicationCommandInteractionDataOptionValue,
            ApplicationCommandOptionType,
        },
        prelude::User,
    },
    prelude::{RwLock, TypeMapKey},
};

pub const LISTENER_RESPONSE_PATH: &str = "data/action response.json";
pub const USER_LISTENER_BLACKLIST_PATH: &str = "data/user listener blacklist.json";

pub struct ListenerResponse;
impl TypeMapKey for ListenerResponse {
    type Value = Arc<RwLock<HashMap<String, String>>>;
}

pub struct UsersBlacklistedFromListener;
impl TypeMapKey for UsersBlacklistedFromListener {
    type Value = Arc<RwLock<HashSet<u64>>>;
}

pub async fn list_listeners(ctx: &Context) -> String {
    let action_response_lock = ctx
        .data
        .read()
        .await
        .get::<ListenerResponse>()
        .unwrap()
        .clone();
    let action_response = action_response_lock.read().await;

    let mut message = String::new();

    for (listener, _) in action_response.iter() {
        message += &format!("{}, ", listener);
    }
    message.pop();
    message.pop();

    return message;
}

pub async fn remove_listener_command(
    ctx: &Context,
    data: &ApplicationCommandInteractionData,
) -> String {
    let listener = data
        .options
        .get(0)
        .expect("expected listener")
        .resolved
        .as_ref()
        .unwrap();
    let action_response_lock = ctx
        .data
        .write()
        .await
        .get::<ListenerResponse>()
        .expect("expected ListenerResponse in TypeMap")
        .clone();

    let mut action_response = action_response_lock.write().await;

    if let ApplicationCommandInteractionDataOptionValue::String(listener) = listener {
        if action_response.contains_key(listener) {
            action_response.remove(listener);
            save_listener_response_to_file(action_response.clone());
            return "Successfully removed the listener".to_string();
        } else {
            return "That listener doesn't exist".to_string();
        }
    }

    return "Something went wrong".to_string();
}

pub async fn set_listener_command(
    ctx: &Context,
    data: &ApplicationCommandInteractionData,
) -> String {
    let listener = data
        .options
        .get(0)
        .expect("expected listener")
        .resolved
        .as_ref()
        .unwrap();
    let response = data
        .options
        .get(1)
        .expect("expected response")
        .resolved
        .as_ref()
        .unwrap();

    if let ApplicationCommandInteractionDataOptionValue::String(listener) = listener {
        if let ApplicationCommandInteractionDataOptionValue::String(response) = response {
            let action_response_lock = ctx
                .data
                .write()
                .await
                .get::<ListenerResponse>()
                .expect("expected ListenerResponse in TypeMap")
                .clone();

            let mut action_response = action_response_lock.write().await;
            action_response.insert(
                listener.to_lowercase().trim().to_string(),
                response.trim().to_string(),
            );
            save_listener_response_to_file(action_response.clone());
            return "Set listener".to_string();
        }
    }
    return "Couldn't set listener".to_string();
}

pub fn save_listener_response_to_file(action_response: HashMap<String, String>) {
    fs::write(
        LISTENER_RESPONSE_PATH,
        serde_json::to_string(&action_response).unwrap(),
    )
    .unwrap();
}

pub async fn blacklist_user_from_listener(ctx: &Context, user: &User) -> String {
    let users_blacklisted_from_listener_lock = ctx
        .data
        .read()
        .await
        .get::<UsersBlacklistedFromListener>()
        .unwrap()
        .clone();

    let mut users_blacklisted_from_listener = users_blacklisted_from_listener_lock.write().await;

    if !users_blacklisted_from_listener.contains(&user.id.0) {
        users_blacklisted_from_listener.insert(user.id.0);
        save_user_listener_blacklist_to_file(users_blacklisted_from_listener.clone());
        return "Added user to the blacklist".to_string();
    } else {
        users_blacklisted_from_listener.remove(&user.id.0);
        save_user_listener_blacklist_to_file(users_blacklisted_from_listener.clone());
        return "Removed user from the blacklist".to_string();
    }
}

pub fn save_user_listener_blacklist_to_file(blacklist: HashSet<u64>) {
    fs::write(
        USER_LISTENER_BLACKLIST_PATH,
        serde_json::to_string(&blacklist).unwrap(),
    )
    .unwrap();
}

pub fn create_listener_commands(
    commands: &mut serenity::builder::CreateApplicationCommands,
) -> &mut serenity::builder::CreateApplicationCommands {
    commands.create_application_command(|command| {
            command.name("setlistener").description(
                "Start a listener for a word or list of words and a response whenever someone says that word",
            )
            .create_option(|option|{
                option.name("listenedword").description("What word to listen for").kind(ApplicationCommandOptionType::String).required(true)
            })
            .create_option(|option|{
                option.name("response").description("What the response should be when the listened word is said")
                .kind(ApplicationCommandOptionType::String)
                .required(true)
            })
        })
        .create_application_command(|command| {
            command.name("removelistener").description("Remove a listener from a word").create_option(|option|{
                option.name("listenedword").description("The word to remove").kind(ApplicationCommandOptionType::String).required(true)
            })
        })
        .create_application_command(|command|{
            command.name("listeners").description("List all of the listeners")
        })
        .create_application_command(|command|{
            command.name("blacklistlistener").description("The bot won't respond to your messages if you trip off a listener")
        })
}
