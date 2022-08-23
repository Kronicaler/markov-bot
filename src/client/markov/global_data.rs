use dashmap::DashSet;
use markov_strings::Markov;
use serenity::prelude::{RwLock, TypeMap, TypeMapKey};
use std::sync::Arc;

pub struct MarkovChain;
impl TypeMapKey for MarkovChain {
    type Value = Arc<RwLock<Markov>>;
}
pub const MARKOV_DATA_SET_PATH: &str = "data/markov data/markov data set.txt";
pub const MARKOV_EXPORT_PATH: &str = "data/markov data/corpus.json";

/// User Ids that the bot will not learn from
pub struct MarkovBlacklistedUser{
    pub user_id: u64,
}

/// Channel Ids that the bot will not learn from
pub struct MarkovBlacklistedChannels;
impl TypeMapKey for MarkovBlacklistedChannels {
    type Value = Arc<DashSet<u64>>;
}
pub const MARKOV_BLACKLISTED_CHANNELS_PATH: &str = "data/markov data/blacklisted channels.json";

pub struct MarkovBlacklistedServer {
    pub server_id: u64,
}

pub async fn get_markov_blacklisted_channels_lock(
    data: &Arc<RwLock<TypeMap>>,
) -> Arc<DashSet<u64>> {
    let markov_blacklisted_channels_lock = data
        .read()
        .await
        .get::<MarkovBlacklistedChannels>()
        .expect("expected MarkovBlacklistedChannels in TypeMap")
        .clone();
    markov_blacklisted_channels_lock
}

pub async fn get_markov_chain_lock(data: &Arc<RwLock<TypeMap>>) -> Arc<RwLock<Markov>> {
    let markov_chain_lock = data
        .read()
        .await
        .get::<MarkovChain>()
        .expect("expected MarkovChain in TypeMap")
        .clone();
    markov_chain_lock
}
