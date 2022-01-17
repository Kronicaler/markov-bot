use serenity::builder::CreateEmbed;
/*
 * voice.rs, LsangnaBoi 2022
 * voice channel functionality
 */
use serenity::utils::Colour;
use serenity::{
    client::Context,
    model::interactions::application_command::{
        ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
    },
};
use songbird::model::id::GuildId;
use songbird::{create_player, input::ytdl_search};

use super::slash_commands::ResponseType;

///play song from youtube
pub async fn play(ctx: &Context, command: &ApplicationCommandInteraction) -> ResponseType {
    //get the guild ID, cache, and query
    let guild_id = command.guild_id.expect("Couldn't get guild ID");
    let user_id = command.user.id;
    let cache = &ctx.cache;
    let query = command
        .data
        .options
        .iter()
        .find(|opt| opt.name == "query")
        .expect("expected input");
    let query = match query.resolved.as_ref().unwrap() {
        ApplicationCommandInteractionDataOptionValue::String(s) => s,
        _ => panic!("expected a string"),
    };

    command
        .create_interaction_response(&ctx.http, |f| {
            f.kind(serenity::model::interactions::InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content("Searching..."))
        })
        .await
        .expect("Couldn't send response");

    //create manager
    let manager = songbird::get(ctx).await.expect("songbird error").clone();

    let guild = cache
        .guild(guild_id)
        .await
        .expect("unable to fetch guild from the cache");

    //get channel_id
    let channel_id = guild
        .voice_states
        .get(&user_id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            return ResponseType::EditWithContent(String::from(
                "You must be in a voice channel to use this command!",
            ));
        }
    };

    //join voice channel
    let (_handle_lock, _success) = manager.join(GuildId(guild_id.0).0, connect_to).await;

    //if the guild is found
    //create audio source
    if let Some(handler_lock) = manager.get(guild_id.0) {
        let mut handler = handler_lock.lock().await;

        //get source from YouTube
        let source = match ytdl_search(query).await {
            Ok(source) => source,
            Err(why) => {
                println!("Err starting source: {:?}", why);
                return ResponseType::EditWithContent(String::from("couldn't source anything"));
            }
        };

        //create embed
        //title
        let title = source.metadata.title.clone().unwrap();
        //channel
        let channel = source.metadata.channel.clone().unwrap();
        //image
        let thumbnail = source.metadata.thumbnail.clone().unwrap();
        //embed
        let url = source.metadata.source_url.clone().unwrap();
        //duration
        let time = source.metadata.duration.unwrap();
        let minutes = time.as_secs() / 60;
        let seconds = time.as_secs() - minutes * 60;
        let duration = format!("{}:{:02}", minutes, seconds);
        //color
        let colour = Colour::from_rgb(149, 8, 2);

        command
            .delete_original_interaction_response(&ctx.http)
            .await
            .expect("Couldn't delete response");

        //add to queue
        let (mut audio, _) = create_player(source);
        audio.set_volume(0.5);
        handler.enqueue(audio);

        return ResponseType::EditWithEmbed(
            CreateEmbed::default()
                .title(title)
                .colour(colour)
                .description(channel)
                .field("duration: ", duration, false)
                .thumbnail(thumbnail)
                .url(url)
                .clone(),
        );

    //if not in a voice channel
    } else {
        ResponseType::EditWithContent(String::from(
            "Must be in a voice channel to use that command!",
        ))
    }
}

///skip the track
pub async fn skip(ctx: &Context, command: &ApplicationCommandInteraction) -> String {
    let cache = &ctx.cache;
    let guild_id = command.guild_id;
    if let Some(_guild) = cache.guild(guild_id.unwrap()).await {
        let manager = songbird::get(ctx)
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();

        if let Some(handler_lock) = manager.get(guild_id.unwrap().0) {
            let handler = handler_lock.lock().await;
            let queue = handler.queue();
            let _ = queue.skip();
            //embed
            let queuesize: usize;
            if handler.queue().is_empty() {
                queuesize = 1;
            } else {
                queuesize = handler.queue().len() - 1;
            }
            let title = format!("Song skipped, {} left in queue.", queuesize);
            let colour = Colour::from_rgb(149, 8, 2);
            let _ = command
                .channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.title(title);
                        e.colour(colour);
                        e
                    });
                    m
                })
                .await;
        } else {
            return String::from("Must be in a voice channel to use that command!");
        }
    }
    String::from("Skipping song...")
}

///stop playing
pub async fn stop(ctx: &Context, command: &ApplicationCommandInteraction) -> String {
    let cache = &ctx.cache;
    let guild_id = command.guild_id;
    if let Some(_guild) = cache.guild(guild_id.unwrap()).await {
        let manager = songbird::get(ctx)
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();

        if let Some(handler_lock) = manager.get(guild_id.unwrap()) {
            let handler = handler_lock.lock().await;
            let queue = handler.queue();
            let _ = queue.stop();
        } else {
            return String::from("Must be in a voice channel to use that command!");
        }
    }
    //embed
    let _ = command
        .channel_id
        .send_message(&ctx.http, |m| {
            let colour = Colour::from_rgb(149, 8, 2);
            m.embed(|e| {
                e.title(String::from("Stopped playing, the queue has been cleared."));
                e.colour(colour);
                e
            });
            m
        })
        .await;
    String::from("stopping...")
}

///current song
pub async fn playing(ctx: &Context, command: &ApplicationCommandInteraction) -> String {
    let cache = &ctx.cache;
    let guild_id = command.guild_id;
    if let Some(_guild) = cache.guild(guild_id.unwrap()).await {
        let manager = songbird::get(ctx)
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();

        //get the queue
        if let Some(handler_lock) = manager.get(guild_id.unwrap()) {
            let handler = handler_lock.lock().await;
            let queue = handler.queue();
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
            assert_eq!(colour.r(), 149);
            assert_eq!(colour.g(), 8);
            assert_eq!(colour.b(), 2);
            assert_eq!(colour.tuple(), (149, 8, 2));
            let _ = command
                .channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.title(title);
                        e.colour(colour);
                        e.description(channel);
                        e.field("duration: ", duration, false);
                        e.thumbnail(thumbnail);
                        e.url(url);
                        e
                    });
                    m
                })
                .await;
        } else {
            return String::from("You must be in a voice channel to use that command!");
        }
    }
    String::from("Fetching current song...")
}

///get the queue
pub async fn queue(ctx: &Context, command: &ApplicationCommandInteraction) -> String {
    let cache = &ctx.cache;
    let guild_id = command.guild_id;
    if let Some(_guild) = cache.guild(guild_id.unwrap()).await {
        let manager = songbird::get(ctx)
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();

        if let Some(handler_lock) = manager.get(guild_id.unwrap()) {
            let handler = handler_lock.lock().await;
            let queue = handler.queue();

            if queue.is_empty() {
                return String::from("The queue is empty!");
            }
            //embed
            let _ = command
                .channel_id
                .send_message(&ctx.http, |m| {
                    //embed
                    let i: usize;
                    if queue.len() < 10 {
                        i = queue.len();
                    } else {
                        i = 10;
                    }
                    //color
                    let colour = Colour::from_rgb(149, 8, 2);
                    assert_eq!(colour.r(), 149);
                    assert_eq!(colour.g(), 8);
                    assert_eq!(colour.b(), 2);
                    assert_eq!(colour.tuple(), (149, 8, 2));
                    m.embed(|e| {
                        e.title("queue");
                        e.title("Current Queue:");
                        e.description(format!("current size: {}", queue.len()));
                        e.color(colour);
                        for i in 0..i {
                            let song = &queue.current_queue().get(i).unwrap().metadata().clone();
                            let channel = &song.channel.as_ref().unwrap();
                            let title = &song.title.as_ref().unwrap();
                            //duration
                            let time = &song.duration.as_ref().unwrap();
                            let minutes = time.as_secs() / 60;
                            let seconds = time.as_secs() - minutes * 60;
                            let duration = format!("{}:{:02}", minutes, seconds);
                            let arg1 = format!("{}. {} | {}", i + 1, title, channel);
                            e.field(arg1, duration, false);
                        }
                        e
                    });
                    m
                })
                .await;
        } else {
            return String::from("You must be in a voice channel to use that command!");
        }
    }
    String::from("getting queue...")
}
