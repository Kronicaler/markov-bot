pub mod commands;
pub mod helper_funcs;
mod loop_song;
mod play;
mod playing;
mod queue;
mod skip;
mod stop;
mod swap;

use std::sync::Arc;
use std::sync::RwLock;

pub use loop_song::loop_song;
pub use play::play;
pub use playing::playing;
pub use queue::change_queue_page;
pub use queue::queue;
use serenity::async_trait;
use serenity::builder::CreateMessage;
use serenity::builder::EditMessage;
use serenity::builder::GetMessages;
use serenity::client::Context;
use serenity::model::id::ChannelId;
use serenity::model::id::GuildId;
use serenity::model::prelude::Message;
use serenity::prelude::Mutex;
use serenity::prelude::TypeMapKey;
pub use skip::skip;
use songbird::input::AuxMetadata;
use songbird::tracks::PlayMode;
use songbird::EventContext;
use songbird::EventHandler;
pub use stop::stop;
pub use swap::swap;

use crate::client::voice::play::create_track_embed;

/*
 * voice.rs, LasagnaBoi 2022
 * voice channel functionality
 */

struct Handler {
    voice_text_channel: ChannelId,
    guild_id: GuildId,
    last_now_playing_msg: Arc<Mutex<Option<Message>>>,
    ctx: Context,
}

#[async_trait]
impl EventHandler for Handler {
    async fn act(&self, ctx: &songbird::EventContext<'_>) -> Option<songbird::Event> {
        let track_event = if let EventContext::Track(e) = ctx {
            Some(*e)
        } else {
            None
        };

        let (track_state, _) = match track_event {
            Some(e) => *e.get(0).unwrap(),
            None => return None,
        };

        if track_state.playing == PlayMode::Stop || track_state.playing == PlayMode::End {
            self.send_or_edit_now_playing().await;
        }

        None
    }
}

impl Handler {
    async fn send_or_edit_now_playing(&self) {
        let songbird = songbird::get(&self.ctx).await.unwrap();

        let call_lock = songbird.get(self.guild_id).unwrap();
        let call = call_lock.lock().await;

        let playing_track = match call.queue().current() {
            Some(e) => e,
            None => return,
        };

        let embed = create_track_embed(
            &playing_track
                .typemap()
                .read()
                .await
                .get::<MyAuxMetadata>()
                .unwrap()
                .read()
                .unwrap()
                .0,
        );

        let mut last_now_playing_msg_ = self.last_now_playing_msg.lock().await;

        match last_now_playing_msg_.clone() {
            Some(mut e) => {
                let msgs_after_now_playing_msg = self
                    .voice_text_channel
                    .messages(&self.ctx.http, GetMessages::new().after(e.id))
                    .await
                    .unwrap();

                if !msgs_after_now_playing_msg.is_empty() {
                    let now_playing_msg = self.send_now_playing_message(embed).await;

                    *last_now_playing_msg_ = Some(now_playing_msg);
                    return;
                }

                e.edit(&self.ctx.http, EditMessage::new().embed(embed))
                    .await
                    .unwrap();

                *last_now_playing_msg_ = Some(e);
            }
            None => {
                let now_playing_msg = self.send_now_playing_message(embed).await;

                *last_now_playing_msg_ = Some(now_playing_msg);
            }
        }
    }

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

#[derive(Clone)]
pub struct MyAuxMetadata(AuxMetadata);

impl TypeMapKey for MyAuxMetadata {
    type Value = Arc<RwLock<MyAuxMetadata>>;
}
