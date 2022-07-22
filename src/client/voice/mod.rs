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

pub use loop_song::loop_song;
pub use play::play;
pub use playing::playing;
pub use queue::edit_queue;
pub use queue::queue;
use serenity::async_trait;
use serenity::client::Context;
use serenity::model::id::ChannelId;
use serenity::prelude::Mutex;
pub use skip::skip;
use songbird::tracks::PlayMode;
use songbird::Call;
use songbird::EventContext;
use songbird::EventHandler;
pub use stop::stop;
pub use swap::swap_songs;

use crate::client::voice::play::create_track_embed;
/*
 * voice.rs, LsangnaBoi 2022
 * voice channel functionality
 */

struct Handler {
    call_lock: Arc<Mutex<Call>>,
    voice_text_channel: ChannelId,
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
            self.send_now_playing_msg().await;
        }

        None
    }
}

impl Handler {
    async fn send_now_playing_msg(&self) {
        let call = self.call_lock.lock().await;
        let playing_track = match call.queue().current() {
            Some(e) => e,
            None => return,
        };
        let embed = create_track_embed(playing_track.metadata());
        self.voice_text_channel
            .send_message(&self.ctx.http, |m| {
                m.content("Now playing").set_embed(embed)
            })
            .await
            .expect("Couldn't send message");
    }
}
