pub mod commands;
pub mod component_interactions;
pub mod helper_funcs;
pub mod loop_song;
pub mod model;
pub mod play;
pub mod play_from_attachment;
pub mod playing;
pub mod queue;
pub mod skip;
pub mod stop;
pub mod swap;

use self::{model::MyAuxMetadata, queue::command_response::get_song_metadata_from_queue};
use super::ComponentIds;
use crate::client::{
    global_data::GetBotState,
    voice::{play::create_track_embed, queue::update_queue_message::update_queue_message},
};
use futures::future::join_all;
use itertools::Itertools;
use serenity::{
    all::{
        ActionRowComponent, ButtonKind, ButtonStyle, Component, Context, CreateComponent,
        CreateEmbed, CreateSelectMenuKind, GenericChannelId, ReactionType,
    },
    async_trait,
    builder::{
        CreateActionRow, CreateButton, CreateMessage, CreateSelectMenu, CreateSelectMenuOption,
        EditMessage,
    },
    model::{
        id::{ChannelId, GuildId},
        prelude::Message,
    },
    small_fixed_array::FixedString,
};
use songbird::{EventHandler, tracks::TrackQueue};
use std::time::Duration;
use std::{borrow::Cow, cmp::min};
use std::{cmp::max, str::FromStr};
use tokio::time::timeout;
use tracing::{Instrument, info, info_span, instrument, warn};

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
        let songbird = self.ctx.bot_state().read().await.songbird.clone();
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
        let channel_id = call.current_channel().unwrap();
        let guild_id = call.current_connection().unwrap().guild_id;
        let voice_channel_members = self
            .ctx
            .cache
            .guild(GuildId::new(guild_id.get()))
            .unwrap()
            .channels
            .get(&ChannelId::new(channel_id.get()))
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
    voice_text_channel: GenericChannelId,
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

        let manager = self.ctx.bot_state().read().await.songbird.clone();

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
    async fn send_now_playing_message(&self, embed: CreateEmbed<'_>) -> Message {
        self.voice_text_channel
            .send_message(
                &self.ctx.http,
                CreateMessage::new()
                    .content("Now playing")
                    .embed(embed)
                    .components(Cow::Owned(vec![set_skip_button_row()])),
            )
            .instrument(info_span!("Sending message"))
            .await
            .expect("Couldn't send message")
    }

    #[instrument]
    async fn update_last_message(&self) {
        let songbird = self.ctx.bot_state().read().await.songbird.clone();

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

        let state_lock = self.ctx.bot_state();

        let last_message = state_lock
            .read()
            .instrument(info_span!("Waiting for voice_messages read lock"))
            .await
            .voice_messages
            .get_last_message_type_in_channel(self.guild_id, &self.ctx)
            .await;

        match last_message {
            model::LastMessageType::NowPlaying(mut message) => {
                let mut buttons = vec![create_skip_button()];
                if message.components.iter().any(|c| {
                    if let Component::ActionRow(c) = c {
                        c.components.iter().any(|c| {
                            if let ActionRowComponent::Button(b) = c
                                && let ButtonKind::NonLink {
                                    custom_id,
                                    style: _,
                                } = &b.data
                                && custom_id == &ComponentIds::Shuffle.to_string()
                            {
                                return true;
                            }
                            false
                        })
                    } else {
                        false
                    }
                }) {
                    buttons.push(create_shuffle_button());
                }

                message
                    .edit(
                        &self.ctx.http,
                        EditMessage::new().embed(embed).components(Cow::Owned(vec![
                            CreateComponent::ActionRow(CreateActionRow::Buttons(Cow::Owned(
                                buttons,
                            ))),
                        ])),
                    )
                    .instrument(info_span!("Sending message"))
                    .await
                    .unwrap();

                state_lock
                    .write()
                    .instrument(info_span!("Waiting for voice_messages write lock"))
                    .await
                    .voice_messages
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

                    state_lock
                        .write()
                        .instrument(info_span!("Waiting for voice_messages write lock"))
                        .await
                        .voice_messages
                        .last_position_in_queue
                        .remove(&self.guild_id)
                        .unwrap();

                    state_lock
                        .write()
                        .instrument(info_span!("Waiting for voice_messages write lock"))
                        .await
                        .voice_messages
                        .last_now_playing
                        .insert(self.guild_id, message);
                } else {
                    let now_playing_msg = self.send_now_playing_message(embed).await;

                    state_lock
                        .write()
                        .instrument(info_span!("Waiting for voice_messages write lock"))
                        .await
                        .voice_messages
                        .last_now_playing
                        .insert(self.guild_id, now_playing_msg);
                }
            }
            model::LastMessageType::None => {
                let now_playing_msg = self.send_now_playing_message(embed).await;

                state_lock
                    .write()
                    .instrument(info_span!("Waiting for voice_messages write lock"))
                    .await
                    .voice_messages
                    .last_now_playing
                    .insert(self.guild_id, now_playing_msg);
            }
        }
    }
}

fn set_skip_button_row<'a>() -> CreateComponent<'a> {
    CreateComponent::ActionRow(CreateActionRow::Buttons(Cow::Owned(vec![
        create_skip_button(),
    ])))
}

pub fn create_skip_button() -> CreateButton<'static> {
    CreateButton::new(ComponentIds::Skip.to_string())
        .label("Skip")
        .style(ButtonStyle::Primary)
}

pub fn create_play_now_button() -> CreateButton<'static> {
    CreateButton::new(ComponentIds::PlayNow.to_string())
        .label("Play now")
        .style(ButtonStyle::Primary)
}

pub fn create_bring_to_front_button() -> CreateButton<'static> {
    CreateButton::new(ComponentIds::BringToFront.to_string())
        .label("Bring to front")
        .style(ButtonStyle::Primary)
}

pub fn create_shuffle_button() -> CreateButton<'static> {
    CreateButton::new(ComponentIds::Shuffle.to_string())
        .label("Shuffle")
        .style(ButtonStyle::Primary)
}

pub fn create_emoji_shuffle_button() -> CreateButton<'static> {
    CreateButton::new(ComponentIds::Shuffle.to_string())
        .emoji(ReactionType::Unicode(FixedString::from_str("🔀").unwrap()))
        .style(ButtonStyle::Primary)
}

pub async fn create_bring_to_front_select_menu(
    queue: &TrackQueue,
    queue_start: usize,
) -> CreateSelectMenu<'_> {
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
        CreateSelectMenuKind::String {
            options: Cow::Owned(options),
        },
    )
    .placeholder("Bring a song to the front of the queue")
}

pub async fn create_play_now_select_menu(
    queue: &TrackQueue,
    queue_start: usize,
) -> CreateSelectMenu<'_> {
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
        CreateSelectMenuKind::String {
            options: Cow::Owned(options),
        },
    )
    .placeholder("Play a song now")
}
