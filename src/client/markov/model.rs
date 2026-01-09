use std::sync::Arc;

use crate::client::global_data::BotState;

use super::file_operations::generate_new_chain_from_msg_file;

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

#[tracing::instrument(skip(state))]
pub async fn replace_markov_chain_lock(state: Arc<BotState>) {
    let new_chain = generate_new_chain_from_msg_file().unwrap();
    state.write().await.markov_chain = new_chain;
}
