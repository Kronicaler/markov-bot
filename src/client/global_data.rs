use crate::client::memes::model::init_memes_data;

use super::{markov::init_markov_data, voice::model::init_voice_data};
use serenity::Client;
use std::error::Error;

pub const HELP_MESSAGE: &str = "All of my commands are slash commands.
/ping: Pong!
/id: gives you the user id of the selected user
/stop-saving-my-messages: tell the bot not to store your messages and not to learn from them
/continue-saving-my-messages: tell the bot to save and learn from your messages
/tag create: create a tag that the bot will listen for and then respond to when it is said
/tag remove: remove a tag
/tag list: list out the current tags
/tag stop-pinging-me: tell the bot not to ping you if you trip off a tag
/tag response-channel: for admins only, set the channel where the bot will respond to tags
/play: play a song from youtube in VC. Accepts both song titles and youtube links
/skip: skip a song
/stop: stop the current song and clear the queue
/playing: check which song is currently playing
/queue: check which songs are in the queue
/queue-shuffle: shuffle all the songs in the queue
/loop: loop the current song
/swap_songs: swap 2 songs positions in the queue
/version: check the version of the bot";

/// Initialize the global data for the client so it can be used from multiple threads.
///
/// If this is the first time the bot is run in the environment it will create the data files with initialized contents
pub async fn init_global_data_for_client(client: &Client) -> Result<(), Box<dyn Error>> {
    let mut data = client.data.write().await;

    if cfg!(debug_assertions) {
        println!("Debugging enabled");
    } else {
        println!("Debugging disabled");
    }

    init_markov_data(&mut data)?;
    init_voice_data(&mut data);
    init_memes_data(&mut data);

    Ok(())
}
