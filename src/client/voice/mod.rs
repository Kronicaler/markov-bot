pub mod commands;
pub mod helper_funcs;
mod loop_song;
pub mod model;
mod play;
mod playing;
mod queue;
mod queue_shuffle;
mod skip;
mod stop;
mod swap;

pub use loop_song::loop_song;
pub use play::play;
pub use playing::playing;
pub use queue::change_queue_page;
pub use queue::queue;
pub use queue_shuffle::shuffle_queue;
use serenity::async_trait;
use serenity::builder::CreateMessage;
use serenity::builder::EditMessage;
use serenity::client::Context;
use serenity::model::id::ChannelId;
use serenity::model::id::GuildId;
use serenity::model::prelude::Message;
pub use skip::skip;
use songbird::EventHandler;
pub use stop::stop;
pub use swap::swap;
use tracing::info_span;
use tracing::instrument;
use tracing::Instrument;

use crate::client::voice::play::create_track_embed;

use self::model::get_voice_messages_lock;
use self::model::MyAuxMetadata;
use self::queue::create_queue_edit_message;
use self::queue::get_queue_start;

/*
 * voice.rs, LasagnaBoi 2022
 * voice channel functionality
 */

struct TrackEndHandler {
    voice_text_channel: ChannelId,
    guild_id: GuildId,
    ctx: Context,
}

struct PeriodicHandler {
    guild_id: GuildId,
    ctx: Context,
}

impl std::fmt::Debug for TrackEndHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackEndHandler")
            .field("voice_text_channel", &self.voice_text_channel)
            .field("guild_id", &self.guild_id)
            .finish()
    }
}

#[async_trait]
impl EventHandler for PeriodicHandler {
    async fn act(&self, _ctx: &songbird::EventContext<'_>) -> Option<songbird::Event> {
        let songbird = songbird::get(&self.ctx).await.unwrap();
        let call_lock = songbird.get(self.guild_id).unwrap();
        let mut call = call_lock.lock().await;

        if self.is_current_voice_channel_empty(&call).await {
            call.queue().stop();
            call.remove_all_global_events();
            call.leave().await.expect("Couldn't leave voice channel");

            return None;
        }

        None
    }
}

impl PeriodicHandler {
    async fn is_current_voice_channel_empty(
        &self,
        call: &tokio::sync::MutexGuard<'_, songbird::Call>,
    ) -> bool {
        let channel_id = call.current_channel().unwrap();

        let voice_channel = self.ctx.cache.guild_channel(channel_id.0).unwrap();

        if voice_channel.members(&self.ctx.cache).unwrap().len() == 1 {
            return true;
        }

        return false;
    }
}

#[async_trait]
impl EventHandler for TrackEndHandler {
    async fn act(&self, _ctx: &songbird::EventContext<'_>) -> Option<songbird::Event> {
        let update_last_message_future = self
            .update_last_message()
            .instrument(info_span!("Updating the 'Now playing' message"));

        let update_queue_message_future = self
            .update_queue_message()
            .instrument(info_span!("Updating the queue message"));

        tokio::join!(update_last_message_future, update_queue_message_future);

        None
    }
}

impl TrackEndHandler {
    async fn send_now_playing_message(&self, embed: serenity::builder::CreateEmbed) -> Message {
        self.voice_text_channel
            .send_message(
                &self.ctx.http,
                CreateMessage::new().content("Now playing").embed(embed),
            )
            .instrument(info_span!("Sending message"))
            .await
            .expect("Couldn't send message")
    }

    #[instrument]
    async fn update_queue_message(&self) {
        let songbird = songbird::get(&self.ctx).await.unwrap();

        let call_lock = songbird.get(self.guild_id).unwrap();

        let voice_messages_lock = get_voice_messages_lock(&self.ctx.data).await;

        let queue_message = voice_messages_lock
            .read()
            .instrument(info_span!("Waiting for voice_messages read lock"))
            .await
            .queue
            .get(&self.guild_id)
            .cloned();

        if let Some(mut queue_message) = queue_message {
            if call_lock
                .lock()
                .instrument(info_span!("Waiting for call lock"))
                .await
                .queue()
                .is_empty()
            {
                queue_message
                    .edit(
                        &self.ctx.http,
                        EditMessage::new().content("The queue is empty!"),
                    )
                    .instrument(info_span!("Sending message"))
                    .await
                    .expect("Error creating interaction response");
                return;
            }
            let queue_start = get_queue_start(&queue_message.content);

            let queue = call_lock
                .lock()
                .instrument(info_span!("Waiting for call lock"))
                .await
                .queue()
                .clone();
            let queue_response = create_queue_edit_message(queue_start, &queue).await;

            queue_message
                .edit(&self.ctx.http, queue_response)
                .instrument(info_span!("Sending message"))
                .await
                .expect("Error creating interaction response");

            voice_messages_lock
                .write()
                .instrument(info_span!("Waiting for voice_messages write lock"))
                .await
                .queue
                .insert(self.guild_id, queue_message);
        }
    }

    #[instrument]
    async fn update_last_message(&self) {
        let songbird = songbird::get(&self.ctx).await.unwrap();

        let call_lock = songbird.get(self.guild_id).unwrap();
        let call = call_lock
            .lock()
            .instrument(info_span!("Waiting for call lock"))
            .await;

        let voice_messages_lock = get_voice_messages_lock(&self.ctx.data).await;

        let next_track = match call.queue().current() {
            Some(e) => e,
            None => return,
        };

        drop(call);

        let track_metadata = next_track
            .typemap()
            .read()
            .await
            .get::<MyAuxMetadata>()
            .unwrap()
            .read()
            .await
            .0
            .clone();

        let embed = create_track_embed(&track_metadata);

        let last_message = voice_messages_lock
            .read()
            .instrument(info_span!("Waiting for voice_messages read lock"))
            .await
            .get_last_message_type_in_channel(self.guild_id, &self.ctx)
            .await;

        match last_message {
            model::LastMessageType::NowPlaying(mut message) => {
                message
                    .edit(&self.ctx.http, EditMessage::new().embed(embed))
                    .instrument(info_span!("Sending message"))
                    .await
                    .unwrap();

                voice_messages_lock
                    .write()
                    .instrument(info_span!("Waiting for voice_messages write lock"))
                    .await
                    .last_now_playing
                    .insert(self.guild_id, message);
            }
            model::LastMessageType::PositionInQueue(mut message) => {
                if track_metadata.source_url.as_ref().unwrap()
                    == message.embeds[0].url.as_ref().unwrap()
                {
                    message
                        .edit(
                            &self.ctx.http,
                            EditMessage::new().embed(embed).content("Now playing"),
                        )
                        .instrument(info_span!("Sending message"))
                        .await
                        .unwrap();

                    voice_messages_lock
                        .write()
                        .instrument(info_span!("Waiting for voice_messages write lock"))
                        .await
                        .last_position_in_queue
                        .remove(&self.guild_id)
                        .unwrap();

                    voice_messages_lock
                        .write()
                        .instrument(info_span!("Waiting for voice_messages write lock"))
                        .await
                        .last_now_playing
                        .insert(self.guild_id, message);
                } else {
                    let now_playing_msg = self.send_now_playing_message(embed).await;

                    voice_messages_lock
                        .write()
                        .instrument(info_span!("Waiting for voice_messages write lock"))
                        .await
                        .last_now_playing
                        .insert(self.guild_id, now_playing_msg);
                }
            }
            model::LastMessageType::None => {
                let now_playing_msg = self.send_now_playing_message(embed).await;

                voice_messages_lock
                    .write()
                    .instrument(info_span!("Waiting for voice_messages write lock"))
                    .await
                    .last_now_playing
                    .insert(self.guild_id, now_playing_msg);
            }
        };
    }
}
