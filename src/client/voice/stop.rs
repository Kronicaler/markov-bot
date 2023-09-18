use serenity::{
    builder::{CreateEmbed, EditInteractionResponse},
    client::Context,
    model::prelude::{interaction::application_command::ApplicationCommandInteraction, Colour},
};

use super::{
    helper_funcs::{is_bot_in_another_voice_channel, voice_channel_not_same_response},
    model::get_queue_data_lock,
};

///stop playing
#[tracing::instrument(skip(ctx), level = "info")]
pub async fn stop(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");
    let guild = guild_id.to_guild_cached(&ctx.cache).map(|g| g.to_owned());

    command.defer(&ctx.http).await.unwrap();

    if let Some(guild) = guild {
        if is_bot_in_another_voice_channel(ctx, &guild, command.user.id) {
            voice_channel_not_same_response(command, ctx).await;
            return;
        }
    }

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        queue.stop();
        let queue_lock = get_queue_data_lock(&ctx.data).await;
        let mut queue = queue_lock.write().await;
        queue
            .filling_queue
            .entry(command.guild_id.unwrap())
            .and_modify(|f| *f = false);
    } else {
        command
            .edit_original_interaction_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .content("Must be in a voice channel to use that command!"),
            )
            .await
            .expect("Error creating interaction response");
        return;
    }

    let colour = Colour::from_rgb(149, 8, 2);
    //embed
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new().embed(
                CreateEmbed::new()
                    .title(String::from("Stopped playing, the queue has been cleared."))
                    .colour(colour),
            ),
        )
        .await
        .expect("Error creating interaction response");
}
