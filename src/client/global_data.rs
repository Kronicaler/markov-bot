use super::{markov::init_markov_data, tags::init_tags_data};
use crate::markov;
use serenity::Client;
use std::error::Error;

pub const HELP_MESSAGE: &str = "All of my commands are slash commands.
/ping: Pong!
/id: gives you the user id of the selected user
/blacklisted-data: lists out the users the bot will not learn from
/stop-saving-my-messages: blacklist yourself if you don't want the bot to store your messages and learn from them
/continue-saving-my-messages: unblacklist yourself if you want the bot to save and learn from your messages
/create-tag: create a tag that the bot will listen for and then respond to when it is said
/remove-tag: remove a tag
/tags: list out the current tags
/blacklist-me-from-tags: blacklist yourself from tags so the bot won't ping you if you trip off a tag
/set-tag-response-channel: for admins only, set the channel the bot will talk in, if you don't want users using the bot anywhere else you'll have to do it with roles
/version: Check the version of the bot";

/// Initialize the global data for the client so it can be used from multiple threads.
///
/// If this is the first time the bot is run in the environment it will create the data files with initialized contents
pub async fn init_global_data_for_client(client: &Client) -> Result<(), Box<dyn Error>> {
    let mut data = client.data.write().await;

    if cfg!(debug_assertions) {
        println!("Debugging enabled");
    } else {
        println!("Debugging disabled");
    };

    let markov = markov::init()?;

    init_markov_data(&mut data, markov)?;

    init_tags_data(data)?;

    Ok(())
}
