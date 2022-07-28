use serenity::{
    client::Context,
    model::prelude::interaction::application_command::ApplicationCommandInteraction, utils::Colour,
};

///current song
pub async fn playing(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = match command.guild_id {
        Some(e) => e,
        None => {
            nothing_playing_response(command, ctx).await;
            return;
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
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
            .create_interaction_response(&ctx.http, |m| {
                m.interaction_response_data(|d| d.embed(|e| create_playing_embed(queue, e)))
            })
            .await
            .expect("Error creating interaction response");
    } else {
        nothing_playing_response(command, ctx).await;
    }
}

async fn nothing_playing_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| d.content("Nothing is currently playing."))
        })
        .await
        .expect("Error creating interaction response");
}

fn create_playing_embed<'a>(
    queue: &songbird::tracks::TrackQueue,
    e: &'a mut serenity::builder::CreateEmbed,
) -> &'a mut serenity::builder::CreateEmbed {
    let song = &queue.current().unwrap().metadata().clone();
    //create embed
    //title
    let title = &song.title.as_ref().unwrap();
    //channel
    let channel = &song.channel.as_ref().unwrap();
    //image
    let thumbnail = &song.thumbnail.as_ref().unwrap();
    //embed
    let url = &song.source_url.as_ref().unwrap();
    //duration
    let time = &song.duration.as_ref().unwrap();
    let minutes = time.as_secs() / 60;
    let seconds = time.as_secs() - minutes * 60;
    let duration = format!("{}:{:02}", minutes, seconds);
    //color
    let colour = Colour::from_rgb(149, 8, 2);
    e.title(title)
        .colour(colour)
        .description(channel)
        .field("duration: ", duration, false)
        .thumbnail(thumbnail)
        .url(url)
}
