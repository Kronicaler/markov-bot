use serde::{Deserialize, Serialize};

#[derive(Hash, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: i32,
    pub listener: String,
    pub response: String,
    pub creator_name: String,
    pub creator_id: i64,
    pub server_id: i64,
}

pub struct TagBlacklistedUser {
    #[allow(dead_code)]
    pub user_id: i64,
}

///Guild, Channel
pub struct TagChannel {
    pub server_id: i64,
    pub channel_id: i64,
}
