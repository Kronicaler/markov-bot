use serenity::{
    client::Context,
    utils::Colour, model::prelude::interaction::application_command::ApplicationCommandInteraction,
};

///current song
pub async fn playing(ctx: &Context, command: &ApplicationCommandInteraction) {
    let cache = &ctx.cache;
    let guild_id = command.guild_id;
    if let Some(_guild) = cache.guild(guild_id.unwrap()) {
        let manager = songbird::get(ctx)
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();

        //get the queue
        if let Some(handler_lock) = manager.get(guild_id.unwrap()) {
            let handler = handler_lock.lock().await;
            let queue = handler.queue();

            if queue.current().is_none() {
                command
                    .create_interaction_response(&ctx.http, |r| {
                        r.interaction_response_data(|d| d.content("Nothing is currently playing."))
                    })
                    .await
                    .expect("Error creating interaction response");
                return;
            }

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
            command
                .create_interaction_response(&ctx.http, |m| {
                    m.interaction_response_data(|d| {
                        d.embed(|e| {
                            e.title(title)
                                .colour(colour)
                                .description(channel)
                                .field("duration: ", duration, false)
                                .thumbnail(thumbnail)
                                .url(url)
                        })
                    })
                })
                .await
                .expect("Error creating interaction response");
        } else {
            command
                .create_interaction_response(&ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content("You must be in a voice channel to use that command!")
                    })
                })
                .await
                .expect("Error creating interaction response");
        }
    }
}
