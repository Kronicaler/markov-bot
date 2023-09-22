use std::{collections::HashMap, sync::Arc};

use serenity::{
    builder::GetMessages,
    model::prelude::{Channel, GuildId, Message},
    prelude::{Context, RwLock, TypeMap, TypeMapKey},
};
use songbird::input::AuxMetadata;

pub fn init_voice_data(data: &mut tokio::sync::RwLockWriteGuard<serenity::prelude::TypeMap>) {
    data.insert::<VoiceMessages>(Arc::new(RwLock::new(VoiceMessages::default())));
    data.insert::<QueueData>(Arc::new(RwLock::new(QueueData::default())));
}

#[derive(Clone)]
pub struct MyAuxMetadata(pub AuxMetadata);

impl TypeMapKey for MyAuxMetadata {
    type Value = Arc<RwLock<MyAuxMetadata>>;
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
}

impl TypeMapKey for QueueData {
    type Value = Arc<RwLock<QueueData>>;
}

impl VoiceMessages {
    #[tracing::instrument(skip(ctx))]
    pub async fn get_last_message_type_in_channel(
        &self,
        guild_id: GuildId,
        ctx: &Context,
    ) -> LastMessageType {
        let now_playing = self.last_now_playing.get(&guild_id);
        let position_in_queue = self.last_position_in_queue.get(&guild_id);

        if let Some(now_playing) = now_playing {
            if is_last_message_in_channel(now_playing, ctx).await {
                return LastMessageType::NowPlaying(now_playing.clone());
            }
        }

        if let Some(position_in_queue) = position_in_queue {
            if is_last_message_in_channel(position_in_queue, ctx).await {
                return LastMessageType::PositionInQueue(position_in_queue.clone());
            }
        }

        LastMessageType::None
    }
}

pub async fn is_last_message_in_channel(message: &Message, ctx: &Context) -> bool {
    let Channel::Guild(channel) = message.channel(&ctx).await.unwrap() else {
        return false;
    };

    return channel
        .messages(&ctx.http, GetMessages::new().after(message.id).limit(1))
        .await
        .unwrap()
        .is_empty();
}

pub enum LastMessageType {
    NowPlaying(Message),
    PositionInQueue(Message),
    None,
}

#[tracing::instrument(skip(data))]
pub async fn get_voice_messages_lock(data: &Arc<RwLock<TypeMap>>) -> Arc<RwLock<VoiceMessages>> {
    data.read()
        .await
        .get::<VoiceMessages>()
        .expect("expected VoiceMessages in TypeMap")
        .clone()
}

#[tracing::instrument(skip(data))]
pub async fn get_queue_data_lock(data: &Arc<RwLock<TypeMap>>) -> Arc<RwLock<QueueData>> {
    data.read()
        .await
        .get::<QueueData>()
        .expect("expected QueueData in TypeMap")
        .clone()
}
