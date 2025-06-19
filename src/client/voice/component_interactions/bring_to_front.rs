use super::super::helper_funcs::is_bot_in_another_voice_channel;
use crate::client::voice::model::HasAuxMetadata;
use crate::client::voice::queue::update_queue_message::update_queue_message;
use serenity::all::{ComponentInteraction, ComponentInteractionDataKind};
use serenity::prelude::Context;
use songbird::Call;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{self, info, Instrument};
use tracing::{info_span, warn};

#[tracing::instrument(skip(ctx))]
pub async fn bring_to_front(ctx: &Context, component: &ComponentInteraction) {
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
        info!("Bot is in another channel");
        return;
    }

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    let Some(call_lock) = manager.get(guild_id) else {
        warn!("Couldn't get call lock");
        return;
    };

    if timeout(Duration::from_secs(30), call_lock.lock())
        .await
        .unwrap()
        .queue()
        .is_empty()
    {
        info!("Queue is empty");
        return;
    }

    match &component.data.kind {
        ComponentInteractionDataKind::Button => {
            bring_to_front_button(component, call_lock.clone()).await;
        }
        ComponentInteractionDataKind::StringSelect { values: _ } => {
            bring_to_front_select_menu(component, call_lock.clone()).await;
        }
        _ => panic!("Unexpected component type"),
    }

    update_queue_message(ctx, component.guild_id.unwrap(), call_lock).await;
}

#[tracing::instrument(skip(call_lock))]
async fn bring_to_front_button(button: &ComponentInteraction, call_lock: Arc<Mutex<Call>>) {
    let song_title = button
        .message
        .embeds
        .first()
        .unwrap()
        .title
        .as_ref()
        .unwrap()
        .clone();

    let queue = timeout(Duration::from_secs(30), call_lock.lock())
        .await
        .unwrap()
        .queue()
        .current_queue();

    for (index, song_to_play_now) in queue.iter().enumerate() {
        let queued_song_title = song_to_play_now.get_aux_metadata().await.title.unwrap();

        if queued_song_title == song_title {
            timeout(Duration::from_secs(30), call_lock.lock())
                .await
                .unwrap()
                .queue()
                .modify_queue(|q| {
                    let song = q.remove(index).unwrap();
                    q.insert(1, song);
                });

            return;
        }
    }

    info!("The song is no longer in the queue");
}

#[tracing::instrument]
async fn bring_to_front_select_menu(
    select_menu: &ComponentInteraction,
    call_lock: Arc<Mutex<Call>>,
) {
    let index: usize = match &select_menu.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => values[0].clone(),
        _ => panic!("unknown select menu kind"),
    }
    .parse()
    .unwrap();

    call_lock
        .lock()
        .instrument(info_span!("waiting for lock"))
        .await
        .queue()
        .modify_queue(|q| {
            let song = q.remove(index).unwrap();
            q.insert(1, song);
        });
}
