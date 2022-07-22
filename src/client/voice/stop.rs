use serenity::{
    client::Context, model::interactions::application_command::ApplicationCommandInteraction,
    utils::Colour,
};

use super::helper_funcs::is_bot_in_another_channel;

///stop playing
pub async fn stop(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");

    if let Some(guild) = guild_id.to_guild_cached(&ctx.cache).await {
        if is_bot_in_another_channel(ctx, &guild, command.user.id) {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content("Must be in the same voice channel to use that command!")
                    })
                })
                .await
                .expect("Error creating interaction response");
            return;
        }
    }

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        queue.stop();
    } else {
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| {
                    d.content("Must be in a voice channel to use that command!")
                })
            })
            .await
            .expect("Error creating interaction response");
        return;
    }

    //embed
    command
        .create_interaction_response(&ctx.http, |m| {
            let colour = Colour::from_rgb(149, 8, 2);
            m.interaction_response_data(|d| {
                d.create_embed(|e| {
                    e.title(String::from("Stopped playing, the queue has been cleared."))
                        .colour(colour)
                })
            });
            m
        })
        .await
        .expect("Error creating interaction response");
}
