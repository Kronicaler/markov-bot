use serenity::{
    builder::{CreateEmbed, EditInteractionResponse},
    client::Context,
    model::prelude::{interaction::application_command::ApplicationCommandInteraction, Colour},
};

use super::MyAuxMetadata;

///current song
pub async fn playing(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = if let Some(e) = command.guild_id {
        e
    } else {
        nothing_playing_response(command, ctx).await;
        return;
    };

    command.defer(&ctx.http).await.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialization.")
        .clone();

    //get the queue
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        if queue.current().is_none() {
            nothing_playing_response(command, ctx).await;
            return;
        }

        command
            .edit_original_interaction_response(
                &ctx.http,
                EditInteractionResponse::new().embed(create_playing_embed(queue).await),
            )
            .await
            .expect("Error creating interaction response");
    } else {
        nothing_playing_response(command, ctx).await;
    }
}

async fn nothing_playing_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new().content("Nothing is currently playing."),
        )
        .await
        .expect("Error creating interaction response");
}

async fn create_playing_embed(
    queue: &songbird::tracks::TrackQueue,
) -> serenity::builder::CreateEmbed {
    let track_handle = queue.current().unwrap();

    let song = track_handle
        .typemap()
        .read()
        .await
        .get::<MyAuxMetadata>()
        .unwrap()
        .read()
        .unwrap()
        .0
        .clone();
    //create embed
    //title
    let title = &song.title.unwrap();
    //channel
    let channel = &song.channel.unwrap();
    //image
    let thumbnail = &song.thumbnail.unwrap();
    //embed
    let url = &song.source_url.unwrap();
    //color
    let colour = Colour::from_rgb(149, 8, 2);

    let time = &song.duration.unwrap();
    let minutes = time.as_secs() / 60;
    let seconds = time.as_secs() % 60;
    let duration = format!("{minutes}:{seconds:02}");
    let position = track_handle.get_info().await.unwrap().position;
    let position = format!("{}:{:02}", position.as_secs() / 60, position.as_secs() % 60);
    let position_to_duration = format!("{position} / {duration}");

    CreateEmbed::new()
        .title(title)
        .colour(colour)
        .description(channel)
        .field("Duration: ", position_to_duration, false)
        .thumbnail(thumbnail)
        .url(url)
}
