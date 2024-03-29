use std::time::Duration;

use super::super::helper_funcs::is_bot_in_another_voice_channel;
use serenity::all::ComponentInteraction;
use serenity::prelude::Context;
use tokio::time::timeout;
use tracing::info_span;
use tracing::{self, Instrument};

#[tracing::instrument(skip(ctx))]
pub async fn skip_button_press(ctx: &Context, button: &ComponentInteraction) {
    button
        .defer(&ctx.http)
        .instrument(info_span!("deferring response"))
        .await
        .unwrap();

    let guild_id = button.guild_id.expect("Couldn't get guild ID");

    if is_bot_in_another_voice_channel(
        ctx,
        &guild_id.to_guild_cached(&ctx.cache).unwrap().clone(),
        button.user.id,
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
    let call = timeout(Duration::from_secs(30), call_lock.lock())
        .await
        .unwrap();

    if call.queue().is_empty() {
        return;
    }

    call.queue().skip().expect("Couldn't skip song");
}
