use serenity::{
    all::{Colour, CommandInteraction},
    builder::{CreateEmbed, EditInteractionResponse},
    client::Context,
};
use tracing::{info_span, Instrument};

use super::MyAuxMetadata;

///current song
#[tracing::instrument(skip(ctx))]
pub async fn playing(ctx: &Context, command: &CommandInteraction) {
    let Some(guild_id) = command.guild_id else {
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
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().embed(create_playing_embed(queue).await),
            )
            .instrument(info_span!("Sending message"))
            .await
            .expect("Error creating interaction response");
    } else {
        nothing_playing_response(command, ctx).await;
    }
}

async fn nothing_playing_response(command: &CommandInteraction, ctx: &Context) {
    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content("Nothing is currently playing."),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

async fn create_playing_embed(
    queue: &songbird::tracks::TrackQueue,
) -> serenity::builder::CreateEmbed {
    let track_handle = queue.current().unwrap();

    let song = track_handle
        .data::<MyAuxMetadata>()
        .0
        .clone();
    //create embed
    //title
    let title = &song.title.unwrap_or_else(|| "Unknown".to_string());
    //channel
    let channel = &song.channel.unwrap_or_else(|| "Unknown".to_string());
    //image
    let thumbnail_option = &song.thumbnail;
    //embed
    let url_option = &song.source_url;
    //color
    let colour = Colour::from_rgb(149, 8, 2);

    let time = &song.duration.unwrap_or_default();
    let minutes = time.as_secs() / 60;
    let seconds = time.as_secs() % 60;
    let duration = format!("{minutes}:{seconds:02}");
    let position = track_handle.get_info().await.unwrap().position;
    let position = format!("{}:{:02}", position.as_secs() / 60, position.as_secs() % 60);
    let position_to_duration = format!("{position} / {duration}");

    let mut embed = CreateEmbed::new()
        .title(title)
        .colour(colour)
        .description(channel)
        .field("Duration: ", position_to_duration, false);

    if let Some(url) = url_option {
        embed = embed.url(url);
    }

    if let Some(thumbnail) = thumbnail_option {
        embed = embed.thumbnail(thumbnail);
    }

    embed
}
