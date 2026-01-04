use markov_str::MarkovChain;
use serenity::prelude::{RwLock, TypeMap, TypeMapKey};
use std::sync::Arc;

use super::file_operations::generate_new_chain_from_msg_file;

pub struct MyMarkovChain;
impl TypeMapKey for MyMarkovChain {
    type Value = Arc<RwLock<MarkovChain>>;
}
pub const MARKOV_DATA_SET_PATH: &str = "data/markov data/markov data set.txt";
pub const MARKOV_EXPORT_PATH: &str = "data/markov data/corpus.json";

/// User Ids that the bot will not learn from
pub struct MarkovBlacklistedUser {
    #[allow(dead_code)]
    pub user_id: i64,
}

/// Channel Ids that the bot will not learn from
pub struct MarkovBlacklistedChannel {
    pub channel_id: i64,
}

// Server Ids that the bot will not learn from
pub struct MarkovBlacklistedServer {
    pub server_id: i64,
}

#[tracing::instrument(skip(data))]
pub async fn get_markov_chain_lock(data: &Arc<RwLock<TypeMap>>) -> Arc<RwLock<MarkovChain>> {
    data.read()
        .await
        .get::<MyMarkovChain>()
        .expect("expected MarkovChain in TypeMap")
        .clone()
}

#[tracing::instrument(skip(data))]
pub async fn replace_markov_chain_lock(data: &Arc<RwLock<TypeMap>>) {
    let mut type_map = data.write().await;

    let markov_chain = type_map.remove::<MyMarkovChain>();

    drop(markov_chain);

    let chain = generate_new_chain_from_msg_file().unwrap();

    type_map.insert::<MyMarkovChain>(Arc::new(RwLock::new(chain)));
}
