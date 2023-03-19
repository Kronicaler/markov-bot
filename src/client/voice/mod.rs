pub mod commands;
pub mod helper_funcs;
mod loop_song;
pub mod model;
mod play;
mod playing;
mod queue;
mod skip;
mod stop;
mod swap;

pub use loop_song::loop_song;
pub use play::play;
pub use playing::playing;
pub use queue::change_queue_page;
pub use queue::queue;
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

use crate::client::voice::play::create_track_embed;

use self::model::get_voice_messages_lock;
use self::model::MyAuxMetadata;

/*
 * voice.rs, LasagnaBoi 2022
 * voice channel functionality
 */

struct TrackEndHandler {
    voice_text_channel: ChannelId,
    guild_id: GuildId,
    ctx: Context,
}

#[async_trait]
impl EventHandler for TrackEndHandler {
    async fn act(&self, _ctx: &songbird::EventContext<'_>) -> Option<songbird::Event> {
        let songbird = songbird::get(&self.ctx).await.unwrap();

        let call_lock = songbird.get(self.guild_id).unwrap();
        let call = call_lock.lock().await;

        let next_track = match call.queue().current() {
            Some(e) => e,
            None => return None,
        };

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

        let voice_messages_lock = get_voice_messages_lock(&self.ctx.data).await;
        let mut voice_messages = voice_messages_lock.write().await;

        let last_message = voice_messages
            .get_last_message_type_in_channel(self.guild_id, &self.ctx)
            .await;

        match last_message {
            model::LastMessageType::NowPlaying(mut message) => {
                message
                    .edit(&self.ctx.http, EditMessage::new().embed(embed))
                    .await
                    .unwrap();

                voice_messages
                    .last_now_playing
                    .insert(self.guild_id, message);
            }
            model::LastMessageType::PositionInQueue(mut message) => {
                match track_metadata.source_url.as_ref().unwrap()
                    == message.embeds[0].url.as_ref().unwrap()
                {
                    true => {
                        message
                            .edit(
                                &self.ctx.http,
                                EditMessage::new().embed(embed).content("Now playing"),
                            )
                            .await
                            .unwrap();

                        voice_messages
                            .last_position_in_queue
                            .remove(&self.guild_id)
                            .unwrap();

                        voice_messages
                            .last_now_playing
                            .insert(self.guild_id, message);
                    }
                    false => {
                        let now_playing_msg = self.send_now_playing_message(embed).await;

                        voice_messages
                            .last_now_playing
                            .insert(self.guild_id, now_playing_msg);
                    }
                }
            }
            model::LastMessageType::None => {
                let now_playing_msg = self.send_now_playing_message(embed).await;

                voice_messages
                    .last_now_playing
                    .insert(self.guild_id, now_playing_msg);
            }
        };

        // if let Some(last_now_playing_msg) = voice_messages.last_now_playing.get_mut(&self.guild_id)
        // {
        //     let last_now_playing_msg_is_last_message_in_channel = self
        //         .voice_text_channel
        //         .messages(
        //             &self.ctx.http,
        //             GetMessages::new().after(last_now_playing_msg.id).limit(1),
        //         )
        //         .await
        //         .unwrap()
        //         .is_empty();

        //     if !last_now_playing_msg_is_last_message_in_channel {
        //         *last_now_playing_msg = self.send_now_playing_message(embed).await;
        //         return None;
        //     }

        //     last_now_playing_msg
        //         .edit(&self.ctx.http, EditMessage::new().embed(embed))
        //         .await
        //         .unwrap();
        // } else {
        //     let now_playing_msg = self.send_now_playing_message(embed).await;

        //     voice_messages
        //         .last_now_playing
        //         .insert(now_playing_msg.guild_id.unwrap(), now_playing_msg);
        // }

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
            .await
            .expect("Couldn't send message")
    }
}
