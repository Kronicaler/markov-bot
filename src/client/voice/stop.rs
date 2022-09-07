use serenity::{
    builder::{CreateEmbed, CreateInteractionResponse, CreateInteractionResponseData},
    client::Context,
    model::prelude::{interaction::application_command::ApplicationCommandInteraction, Colour},
};

use super::helper_funcs::is_bot_in_another_channel;

///stop playing
pub async fn stop(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");
    let guild = guild_id
        .to_guild_cached(&ctx.cache)
        .and_then(|g| Some(g.to_owned()));

    if let Some(guild) = guild {
        if is_bot_in_another_channel(ctx, &guild, command.user.id) {
            command
                .create_interaction_response(
                    &ctx.http,
                    CreateInteractionResponse::new().interaction_response_data(
                        CreateInteractionResponseData::new()
                            .content("Must be in the same voice channel to use that command!"),
                    ),
                )
                .await
                .expect("Error creating interaction response");
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
    } else {
        command
            .create_interaction_response(
                &ctx.http,
                CreateInteractionResponse::new().interaction_response_data(
                    CreateInteractionResponseData::new()
                        .content("Must be in a voice channel to use that command!"),
                ),
            )
            .await
            .expect("Error creating interaction response");
        return;
    }

    let colour = Colour::from_rgb(149, 8, 2);
    //embed
    command
        .create_interaction_response(
            &ctx.http,
            CreateInteractionResponse::new().interaction_response_data(
                CreateInteractionResponseData::new().embed(
                    CreateEmbed::new()
                        .title(String::from("Stopped playing, the queue has been cleared."))
                        .colour(colour),
                ),
            ),
        )
        .await
        .expect("Error creating interaction response");
}
