use markov_strings::Markov;
use serenity::prelude::{RwLock, TypeMap, TypeMapKey};
use std::sync::Arc;

use super::{create_default_chain_from_export, file_operations::generate_new_corpus_from_msg_file};

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

// Server Ids that the bot will not learn from
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

pub async fn replace_markov_chain_lock(data: &Arc<RwLock<TypeMap>>) {
    let mut type_map = data.write().await;

    let markov_chain = type_map.remove::<MarkovChain>();

    drop(markov_chain);

    let corpus = generate_new_corpus_from_msg_file().unwrap();

    type_map.insert::<MarkovChain>(Arc::new(RwLock::new(create_default_chain_from_export(
        corpus,
    ))));
}
