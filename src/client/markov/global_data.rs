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
pub struct MarkovBlacklistedUser {
    pub user_id: u64,
}

/// Channel Ids that the bot will not learn from
pub struct MarkovBlacklistedChannel {
    pub channel_id: u64,
}

pub struct MarkovBlacklistedServer {
    pub server_id: u64,
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
