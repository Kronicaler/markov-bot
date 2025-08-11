pub mod commands;
pub mod component_interactions;
pub mod helper_funcs;
mod loop_song;
pub mod model;
mod play;
mod play_from_attachment;
mod playing;
pub mod queue;
mod skip;
mod stop;
mod swap;

use self::model::get_voice_messages_lock;
use self::model::MyAuxMetadata;
use self::queue::command_response::get_song_metadata_from_queue;
use super::ComponentIds;
use crate::client::voice::play::create_track_embed;
use crate::client::voice::queue::update_queue_message::update_queue_message;
use futures::future::join_all;
use itertools::Itertools;
pub use loop_song::loop_song;
pub use play::play;
pub use play_from_attachment::play_from_attachment;
pub use playing::playing;
use serenity::all::ActionRowComponent;
use serenity::all::ButtonKind;
use serenity::all::ButtonStyle;
use serenity::all::CreateSelectMenuKind;
use serenity::all::ReactionType;
use serenity::async_trait;
use serenity::builder::CreateActionRow;
use serenity::builder::CreateButton;
use serenity::builder::CreateMessage;
use serenity::builder::CreateSelectMenu;
use serenity::builder::CreateSelectMenuOption;
use serenity::builder::EditMessage;
use serenity::client::Context;
use serenity::model::id::ChannelId;
use serenity::model::id::GuildId;
use serenity::model::prelude::Message;
pub use skip::skip;
use songbird::tracks::TrackQueue;
use songbird::EventHandler;
use std::cmp::max;
use std::cmp::min;
use std::time::Duration;
pub use stop::stop;
pub use swap::swap;
use tokio::time::timeout;
use tracing::info;
use tracing::info_span;
use tracing::instrument;
use tracing::warn;
use tracing::Instrument;

/*
 * voice.rs, LasagnaBoi 2022
 * voice channel functionality
 */

struct PeriodicHandler {
    guild_id: GuildId,
    ctx: Context,
}

#[async_trait]
impl EventHandler for PeriodicHandler {
    async fn act(&self, _ctx: &songbird::EventContext<'_>) -> Option<songbird::Event> {
        let songbird = songbird::get(&self.ctx).await.unwrap();
        let call_lock = songbird.get(self.guild_id).unwrap();
        let mut call = timeout(Duration::from_secs(30), call_lock.lock())
            .await
            .unwrap();

        if self.is_current_voice_channel_empty(&call) {
            call.queue().stop();
            call.remove_all_global_events();
            call.leave().await.expect("Couldn't leave voice channel");

            return None;
        }

        None
    }
}

impl PeriodicHandler {
    fn is_current_voice_channel_empty(
        &self,
        call: &tokio::sync::MutexGuard<'_, songbird::Call>,
    ) -> bool {
        let channel_id = ChannelId::new(call.current_channel().unwrap().0.get());
        let guild_id = call.current_connection().unwrap().guild_id;
        let voice_channel_members = self
            .ctx
            .cache
            .guild(guild_id.0)
            .unwrap()
            .channels
            .get(&channel_id)
            .unwrap()
            .members(&self.ctx.cache)
            .unwrap();

        if voice_channel_members.len() == 1 {
            return true;
        }

        false
    }
}

struct TrackStartHandler {
    voice_text_channel: ChannelId,
    guild_id: GuildId,
    ctx: Context,
}

impl std::fmt::Debug for TrackStartHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackEndHandler")
            .field("voice_text_channel", &self.voice_text_channel)
            .field("guild_id", &self.guild_id)
            .field("ctx", &Option::<i32>::None)
            .finish()
    }
}

#[async_trait]
impl EventHandler for TrackStartHandler {
    async fn act(&self, _ctx: &songbird::EventContext<'_>) -> Option<songbird::Event> {
        let update_last_message_future = self
            .update_last_message()
            .instrument(info_span!("Updating the 'Now playing' message"));

        let manager = songbird::get(&self.ctx)
            .await
            .expect("Songbird Voice client placed in at initialization.")
            .clone();

        let Some(call_lock) = manager.get(self.guild_id) else {
            warn!("Couldn't get call lock");
            return None;
        };

        let update_queue_message_future = update_queue_message(&self.ctx, self.guild_id, call_lock)
            .instrument(info_span!("Updating the queue message"));

        tokio::join!(update_last_message_future, update_queue_message_future);

        None
    }
}

mod modname {}

impl TrackStartHandler {
    #[instrument]
    async fn send_now_playing_message(&self, embed: serenity::builder::CreateEmbed) -> Message {
        self.voice_text_channel
            .send_message(
                &self.ctx.http,
                CreateMessage::new()
                    .content("Now playing")
                    .embed(embed)
                    .components(vec![set_skip_button_row()]),
            )
            .instrument(info_span!("Sending message"))
            .await
            .expect("Couldn't send message")
    }

    #[instrument]
    async fn update_last_message(&self) {
        let songbird = songbird::get(&self.ctx).await.unwrap();

        let call_lock = songbird.get(self.guild_id).unwrap();
        let call = call_lock
            .lock()
            .instrument(info_span!("Waiting for call lock"))
            .await;

        let Some(next_track) = call.queue().current() else {
            info!("There is no currently playing track");
            return;
        };

        drop(call);

        let track_metadata = next_track.data::<MyAuxMetadata>().aux_metadata.clone();

        let embed = create_track_embed(&track_metadata);

        let voice_messages_lock = get_voice_messages_lock(&self.ctx.data).await;

        let last_message = voice_messages_lock
            .read()
            .instrument(info_span!("Waiting for voice_messages read lock"))
            .await
            .get_last_message_type_in_channel(self.guild_id, &self.ctx)
            .await;

        match last_message {
            model::LastMessageType::NowPlaying(mut message) => {
                let mut buttons = vec![create_skip_button()];
                if message.components.iter().any(|c| {
                    c.components.iter().any(|c| {
                        if let ActionRowComponent::Button(b) = c {
                            if let ButtonKind::NonLink {
                                custom_id,
                                style: _,
                            } = &b.data
                            {
                                if custom_id == &ComponentIds::Shuffle.to_string() {
                                    return true;
                                }
                            }
                        }
                        false
                    })
                }) {
                    buttons.push(create_shuffle_button());
                }

                message
                    .edit(
                        &self.ctx.http,
                        EditMessage::new()
                            .embed(embed)
                            .components(vec![CreateActionRow::Buttons(buttons)]),
                    )
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
                            EditMessage::new()
                                .embed(embed)
                                .content("Now playing")
                                .components(vec![set_skip_button_row()]),
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
        }
    }
}

fn set_skip_button_row() -> CreateActionRow {
    CreateActionRow::Buttons(vec![create_skip_button()])
}

pub fn create_skip_button() -> CreateButton {
    CreateButton::new(ComponentIds::Skip.to_string())
        .label("Skip")
        .style(ButtonStyle::Primary)
}

pub fn create_play_now_button() -> CreateButton {
    CreateButton::new(ComponentIds::PlayNow.to_string())
        .label("Play now")
        .style(ButtonStyle::Primary)
}

pub fn create_bring_to_front_button() -> CreateButton {
    CreateButton::new(ComponentIds::BringToFront.to_string())
        .label("Bring to front")
        .style(ButtonStyle::Primary)
}

pub fn create_shuffle_button() -> CreateButton {
    CreateButton::new(ComponentIds::Shuffle.to_string())
        .label("Shuffle")
        .style(ButtonStyle::Primary)
}

pub fn create_emoji_shuffle_button() -> CreateButton {
    CreateButton::new(ComponentIds::Shuffle.to_string())
        .emoji(ReactionType::Unicode("🔀".to_string()))
        .style(ButtonStyle::Primary)
}

pub async fn create_bring_to_front_select_menu(
    queue: &TrackQueue,
    queue_start: usize,
) -> CreateSelectMenu {
    let queue_start_index = max(queue_start - 1, 1);

    let number_of_songs = if queue_start_index == 1 { 9 } else { 10 };

    let options = (queue_start_index..(min(queue_start_index + number_of_songs, queue.len())))
        .map(|i| async move {
            CreateSelectMenuOption::new(
                get_song_metadata_from_queue(queue, i)
                    .await
                    .0
                    .chars()
                    .take(100)
                    .collect::<String>(),
                i.to_string(),
            )
        })
        .collect_vec();
    let mut options = join_all(options).await;

    if options.is_empty() {
        options.push(CreateSelectMenuOption::new("Queue isn't big enough", "-1"));
    }

    CreateSelectMenu::new(
        ComponentIds::BringToFrontMenu.to_string(),
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Bring a song to the front of the queue")
}

pub async fn create_play_now_select_menu(
    queue: &TrackQueue,
    queue_start: usize,
) -> CreateSelectMenu {
    let queue_start_index = max(queue_start - 1, 1);

    let number_of_songs = if queue_start_index == 1 { 9 } else { 10 };

    let options = (queue_start_index..(min(queue_start_index + number_of_songs, queue.len())))
        .map(|i| async move {
            CreateSelectMenuOption::new(
                get_song_metadata_from_queue(queue, i)
                    .await
                    .0
                    .chars()
                    .take(100)
                    .collect::<String>(),
                i.to_string(),
            )
        })
        .collect_vec();
    let mut options = join_all(options).await;

    if options.is_empty() {
        options.push(CreateSelectMenuOption::new("Queue isn't big enough", "-1"));
    }

    CreateSelectMenu::new(
        ComponentIds::PlayNowMenu.to_string(),
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Play a song now")
}
