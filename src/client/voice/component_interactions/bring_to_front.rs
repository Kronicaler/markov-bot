use super::super::helper_funcs::is_bot_in_another_voice_channel;
use crate::client::voice::model::HasAuxMetadata;
use crate::client::voice::queue::update_queue_message::update_queue_message;
use serenity::model::prelude::component::ComponentType;
use serenity::model::prelude::interaction::message_component::MessageComponentInteraction;
use serenity::prelude::Context;
use songbird::Call;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{self, info, Instrument};
use tracing::{info_span, warn};

#[tracing::instrument(skip(ctx))]
pub async fn bring_to_front(ctx: &Context, component: &MessageComponentInteraction) {
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

    if call_lock.lock().await.queue().is_empty() {
        info!("Queue is empty");
        return;
    }

    match component.data.component_type {
        ComponentType::Button => bring_to_front_button(component, call_lock).await,
        ComponentType::SelectMenu => bring_to_front_select_menu(component, call_lock).await,
        _ => panic!("Unexpected component type"),
    }

    update_queue_message(ctx, component.guild_id.unwrap()).await;
}

async fn bring_to_front_button(button: &MessageComponentInteraction, call_lock: Arc<Mutex<Call>>) {
    let song_title = button
        .message
        .embeds
        .get(0)
        .unwrap()
        .title
        .as_ref()
        .unwrap()
        .clone();

    let queue = call_lock.lock().await.queue().current_queue();

    for (index, song_to_play_now) in queue.iter().enumerate() {
        let queued_song_title = song_to_play_now.get_aux_metadata().await.title.unwrap();

        if queued_song_title == song_title {
            call_lock.lock().await.queue().modify_queue(|q| {
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
    select_menu: &MessageComponentInteraction,
    call_lock: Arc<Mutex<Call>>,
) {
    let index: usize = select_menu.data.values[0].parse().unwrap();

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
