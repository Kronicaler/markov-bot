use crate::client::{
    markov::MARKOV_STATE_SIZE,
    voice::model::{QueueData, VoiceMessages},
};

use super::markov::init_markov_data;
use markov_str::MarkovChain;
use regex::Regex;
use serenity::all::Context;
use songbird::{
    Config, Songbird,
    driver::retry::{Retry, Strategy},
};
use std::sync::Arc;
use tokio::sync::RwLock;

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

pub type BotState = RwLock<InnerBotState>;
pub struct InnerBotState {
    pub markov_chain: MarkovChain,
    pub voice_messages: VoiceMessages,
    pub queue_data: QueueData,
    pub songbird: Arc<Songbird>,
}

impl Default for InnerBotState {
    fn default() -> Self {
        Self {
            markov_chain: MarkovChain::new(
                MARKOV_STATE_SIZE,
                Regex::new(markov_str::WORD_REGEX).unwrap(),
            ),
            voice_messages: Default::default(),
            queue_data: Default::default(),
            songbird: Songbird::serenity(),
        }
    }
}

/// Initialize the global data for the client so it can be used from multiple threads.
///
/// If this is the first time the bot is run in the environment it will create the data files with initialized contents
pub async fn init_bot_state() -> anyhow::Result<BotState> {
    if cfg!(debug_assertions) {
        println!("Debugging enabled");
    } else {
        println!("Debugging disabled");
    }

    let songbird_config = Config::default()
        .driver_retry(Retry {
            retry_limit: Some(60),
            strategy: Strategy::Every(std::time::Duration::from_secs(2)),
        })
        .preallocated_tracks(2);
    let songbird = songbird::Songbird::serenity();
    songbird.set_config(songbird_config);

    let markov_chain = init_markov_data()?;

    let bot_state = InnerBotState {
        markov_chain,
        songbird,
        ..Default::default()
    };

    Ok(RwLock::new(bot_state))
}

pub trait GetBotState {
    fn bot_state(&self) -> Arc<BotState>;
}

impl GetBotState for Context {
    fn bot_state(&self) -> Arc<BotState> {
        self.data::<BotState>()
    }
}
