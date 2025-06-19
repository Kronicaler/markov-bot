use super::super::helper_funcs::is_bot_in_another_voice_channel;
use crate::client::voice::model::HasAuxMetadata;
use serenity::all::{ComponentInteraction, ComponentInteractionDataKind};
use serenity::prelude::Context;
use songbird::Call;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{self, Instrument};
use tracing::{info, info_span};

#[tracing::instrument(skip(ctx))]
pub async fn play_now(ctx: &Context, component: &ComponentInteraction) {
    component
        .defer(&ctx.http)
        .instrument(info_span!("deferring response"))
        .await
        .unwrap();

    let guild_id = component.guild_id.expect("Couldn't get guild ID");

    if is_bot_in_another_voice_channel(
        ctx,
        &guild_id.to_guild_cached(&ctx.cache).unwrap().clone(),
        component.user.id,
    ) {
        return;
    }

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    let Some(call_lock) = manager.get(guild_id) else {
        return;
    };

    if timeout(Duration::from_secs(30), call_lock.lock())
        .await
        .unwrap()
        .queue()
        .is_empty()
    {
        return;
    }

    match &component.data.kind {
        ComponentInteractionDataKind::Button => play_now_button(component, call_lock).await,
        ComponentInteractionDataKind::StringSelect { values: _ } => {
            play_now_select_menu(component, call_lock).await;
        }
        _ => panic!("Unknown interaction"),
    }
}

#[tracing::instrument(skip(call_lock))]
async fn play_now_button(button: &ComponentInteraction, call_lock: Arc<Mutex<Call>>) {
    let song_title = button
        .message
        .embeds
        .first()
        .unwrap()
        .title
        .as_ref()
        .unwrap()
        .clone();

    let call = timeout(Duration::from_secs(30), call_lock.lock())
        .await
        .unwrap();

    for (index, song_to_play_now) in call.queue().current_queue().iter().enumerate() {
        let queued_song_title = song_to_play_now.get_aux_metadata().await.title.unwrap();

        if queued_song_title == song_title {
            call.queue().modify_queue(|q| {
                let Some(playing_song) = q.front() else {
                    return;
                };

                _ = playing_song.pause();
                _ = playing_song.seek(Duration::from_secs(0));

                let song_to_play_now = q.remove(index).unwrap();

                q.push_front(song_to_play_now);

                let song_to_play_now = q.front().unwrap();

                song_to_play_now.play().unwrap();
            });

            return;
        }
    }
    info!("The song is no longer in the queue");
}

#[tracing::instrument(skip(call_lock))]
async fn play_now_select_menu(select_menu: &ComponentInteraction, call_lock: Arc<Mutex<Call>>) {
    let index: usize = match &select_menu.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => values[0].clone().parse().unwrap(),
        _ => panic!("unknown play now select menu"),
    };

    timeout(Duration::from_secs(30), call_lock.lock())
        .await
        .unwrap()
        .queue()
        .modify_queue(|q| {
            let Some(playing_song) = q.front() else {
                return;
            };

            _ = playing_song.pause();
            _ = playing_song.seek(Duration::from_secs(0));

            let song_to_play_now = q.remove(index).unwrap();

            q.push_front(song_to_play_now);

            let song_to_play_now = q.front().unwrap();

            song_to_play_now.play().unwrap();
        });
}
