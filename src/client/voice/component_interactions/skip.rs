use std::time::Duration;

use super::super::helper_funcs::is_bot_in_another_voice_channel;
use anyhow::Context as AnyhowContext;
use serenity::all::ComponentInteraction;
use serenity::prelude::Context;
use tokio::time::timeout;
use tracing::info_span;
use tracing::{self, Instrument};

#[tracing::instrument(skip(ctx))]
pub async fn skip_button_press(ctx: &Context, button: &ComponentInteraction) -> anyhow::Result<()> {
    button
        .defer(&ctx.http)
        .instrument(info_span!("deferring response"))
        .await?;

    let guild_id = button.guild_id.context("Couldn't get guild ID")?;

    if is_bot_in_another_voice_channel(
        ctx,
        &guild_id
            .to_guild_cached(&ctx.cache)
            .context("couldn't get guild from guild id")?
            .clone(),
        button.user.id,
    ) {
        return Ok(());
    }

    let manager = songbird::get(ctx)
        .await
        .context("Songbird Voice client placed in at initialization.")?
        .clone();

    let Some(call_lock) = manager.get(guild_id) else {
        return Ok(());
    };
    let call = timeout(Duration::from_secs(30), call_lock.lock()).await?;

    if call.queue().is_empty() {
        return Ok(());
    }

    call.queue().skip().context("Couldn't skip song")?;

    Ok(())
}
