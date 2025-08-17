use serde::{Deserialize, Serialize};

#[derive(Hash, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: u64,
    pub listener: String,
    pub response: String,
    pub creator_name: String,
    pub creator_id: u64,
    pub server_id: u64,
}

pub struct TagBlacklistedUser {
    #[allow(dead_code)]
    pub user_id: u64,
}

///Guild, Channel
pub struct TagChannel {
    pub server_id: u64,
    pub channel_id: u64,
}
