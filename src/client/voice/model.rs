use std::{collections::HashMap, sync::Arc, time::Duration};

use serenity::{
    async_trait,
    builder::GetMessages,
    model::prelude::{Channel, GuildId, Message},
    prelude::{Context, RwLock, TypeMap, TypeMapKey},
};
use songbird::{
    input::AuxMetadata,
    tracks::{Queued, TrackHandle},
};
use tokio::time::timeout;

use crate::client::voice::skip::SkipType;

pub fn init_voice_data(data: &mut tokio::sync::RwLockWriteGuard<serenity::prelude::TypeMap>) {
    data.insert::<VoiceMessages>(Arc::new(RwLock::new(VoiceMessages::default())));
    data.insert::<QueueData>(Arc::new(RwLock::new(QueueData::default())));
}

#[derive(Clone, Default)]
pub struct MyAuxMetadata {
    pub aux_metadata: AuxMetadata,
    pub queued_by: String,
}

impl TypeMapKey for MyAuxMetadata {
    type Value = Arc<RwLock<MyAuxMetadata>>;
}

#[async_trait]
pub trait HasAuxMetadata {
    async fn get_aux_metadata(&self) -> AuxMetadata;
}

#[async_trait]
impl HasAuxMetadata for Queued {
    async fn get_aux_metadata(&self) -> AuxMetadata {
        self.data::<MyAuxMetadata>().aux_metadata.clone()
    }
}

#[async_trait]
impl HasAuxMetadata for TrackHandle {
    async fn get_aux_metadata(&self) -> AuxMetadata {
        self.data::<MyAuxMetadata>().aux_metadata.clone()
    }
}

#[derive(Clone, Default, Debug)]
pub struct VoiceMessages {
    pub last_now_playing: HashMap<GuildId, Message>,
    pub last_position_in_queue: HashMap<GuildId, Message>,
    pub queue: HashMap<GuildId, Message>,
}

impl TypeMapKey for VoiceMessages {
    type Value = Arc<RwLock<VoiceMessages>>;
}

#[derive(Clone, Default)]
pub struct QueueData {
    pub filling_queue: HashMap<GuildId, bool>,
    pub shuffle_queue: HashMap<GuildId, bool>,
    pub skip_queue: HashMap<GuildId, (SkipType, i64)>,
}

impl TypeMapKey for QueueData {
    type Value = Arc<RwLock<QueueData>>;
}

impl VoiceMessages {
    #[tracing::instrument(skip(self, ctx,))]
    pub async fn get_last_message_type_in_channel(
        &self,
        guild_id: GuildId,
        ctx: &Context,
    ) -> LastMessageType {
        let now_playing = self.last_now_playing.get(&guild_id);
        let position_in_queue = self.last_position_in_queue.get(&guild_id);

        if let Some(now_playing) = now_playing
            && is_last_message_in_channel(now_playing, ctx).await
        {
            return LastMessageType::NowPlaying(now_playing.clone());
        }

        if let Some(position_in_queue) = position_in_queue
            && is_last_message_in_channel(position_in_queue, ctx).await
        {
            return LastMessageType::PositionInQueue(position_in_queue.clone());
        }

        LastMessageType::None
    }
}

pub async fn is_last_message_in_channel(message: &Message, ctx: &Context) -> bool {
    let Channel::Guild(channel) = message.channel(&ctx).await.unwrap() else {
        return false;
    };

    channel
        .messages(&ctx.http, GetMessages::new().after(message.id).limit(1))
        .await
        .unwrap()
        .is_empty()
}

pub enum LastMessageType {
    NowPlaying(Message),
    PositionInQueue(Message),
    None,
}

#[tracing::instrument(skip(data))]
pub async fn get_voice_messages_lock(data: &Arc<RwLock<TypeMap>>) -> Arc<RwLock<VoiceMessages>> {
    timeout(Duration::from_secs(30), data.read())
        .await
        .unwrap()
        .get::<VoiceMessages>()
        .expect("expected VoiceMessages in TypeMap")
        .clone()
}

#[tracing::instrument(skip(data))]
pub async fn get_queue_data_lock(data: &Arc<RwLock<TypeMap>>) -> Arc<RwLock<QueueData>> {
    timeout(Duration::from_secs(30), data.read())
        .await
        .unwrap()
        .get::<QueueData>()
        .expect("expected QueueData in TypeMap")
        .clone()
}
