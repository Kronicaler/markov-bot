use serenity::model::id::GuildId;
use serenity::model::prelude::VoiceState;
use serenity::utils::Colour;
use serenity::{
    client::Context,
    model::interactions::application_command::{
        ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
    },
};
use songbird::{create_player, input::ytdl_search};

/*
 * voice.rs, LsangnaBoi 2022
 * voice channel functionality
 */

///play song from youtube
pub async fn play(ctx: &Context, command: &ApplicationCommandInteraction) {
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
            f.interaction_response_data(|message| message.content("Searching..."))
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
            command
                .edit_original_interaction_response(&ctx.http, |r| {
                    r.content("You must be in a voice channel to use this command!")
                })
                .await
                .expect("Error creating interaction response");
            return;
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
                command
                    .edit_original_interaction_response(&ctx.http, |r| {
                        r.content("Coulnd't find the video on Youtube")
                    })
                    .await
                    .expect("Error creating interaction response");
                return;
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

        let content = if handler.queue().is_empty() {
            "Playing"
        } else {
            "Queued up"
        };

        command
            .edit_original_interaction_response(&ctx.http, |r| {
                r.create_embed(|e| {
                    e.title(title)
                        .colour(colour)
                        .description(channel)
                        .field("duration: ", duration, false)
                        .thumbnail(thumbnail)
                        .url(url)
                })
                .content(content)
            })
            .await
            .expect("Error creating interaction response");
        //add to queue
        let (mut audio, _) = create_player(source);
        audio.set_volume(0.5);
        handler.enqueue(audio);

        //if not in a voice channel
    } else {
        command
            .edit_original_interaction_response(&ctx.http, |r| {
                r.content("Must be in a voice channel to use that command!")
            })
            .await
            .expect("Error creating interaction response");
    }
}

/// Skip the track
pub async fn skip(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    // Get call
    let call_lock = manager.get(guild_id.0);
    if call_lock.is_none() {
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| {
                    d.content("Must be in a voice channel to use that command!")
                })
            })
            .await
            .expect("Error creating interaction response");
    }
    let call_lock = call_lock.expect("Couldn't get handler lock");
    let call = call_lock.lock().await;

    if call.queue().len() == 0 {
        command
            .create_interaction_response(&ctx.http, |r| {
                r.interaction_response_data(|d| d.content("The queue is empty."))
            })
            .await
            .expect("Couldn't create response");
        return;
    }

    call.queue().skip().expect("Couldn't skip queue");

    // Embed
    let title = format!("Song skipped, {} left in queue.", call.queue().len() - 1);
    let colour = Colour::from_rgb(149, 8, 2);

    command
        .create_interaction_response(&ctx.http, |m| {
            m.interaction_response_data(|d| d.create_embed(|e| e.title(title).colour(colour)))
        })
        .await
        .expect("Error creating interaction response");
}

///stop playing
pub async fn stop(ctx: &Context, command: &ApplicationCommandInteraction) {
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

///current song
pub async fn playing(ctx: &Context, command: &ApplicationCommandInteraction) {
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
            command
                .create_interaction_response(&ctx.http, |m| {
                    m.interaction_response_data(|d| {
                        d.create_embed(|e| {
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

///get the queue
pub async fn queue(ctx: &Context, command: &ApplicationCommandInteraction) {
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
                command
                    .create_interaction_response(&ctx.http, |r| {
                        r.interaction_response_data(|d| d.content("The queue is empty!"))
                    })
                    .await
                    .expect("Error creating interaction response");
                return;
            }
            //embed
            command
                .create_interaction_response(&ctx.http, |m| {
                    //embed
                    let i: usize;
                    if queue.len() < 10 {
                        i = queue.len();
                    } else {
                        i = 10;
                    }
                    //color
                    let colour = Colour::from_rgb(149, 8, 2);
                    m.interaction_response_data(|d| {
                        d.create_embed(|e| {
                            e.title("queue")
                                .title("Current Queue:")
                                .description(format!("current size: {}", queue.len()))
                                .color(colour);
                            for i in 0..i {
                                let song =
                                    &queue.current_queue().get(i).unwrap().metadata().clone();
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
            return;
        }
    }
}

pub async fn leave_if_alone(
    old: Option<VoiceState>,
    ctx: Context,
    guild_id_option: Option<GuildId>,
) {
    // If a user joined a channel
    if old.is_none() {
        return;
    }

    let old = old.unwrap();

    let manager = songbird::get(&ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let call_mutex = manager.get(guild_id_option.unwrap());
    if call_mutex.is_none() {
        return;
    }
    let call_mutex = call_mutex.unwrap();

    let mut call = call_mutex.lock().await;

    let bot_voice_channel = call.current_channel();
    if bot_voice_channel.is_none() {
        return;
    }
    let bot_voice_channel = bot_voice_channel.unwrap();

    let guild = guild_id_option
        .unwrap()
        .to_guild_cached(&ctx.cache)
        .await
        .unwrap();

    let changed_voice_channel = guild.channels.get(&old.channel_id.unwrap()).unwrap();

    if changed_voice_channel.id.0 != bot_voice_channel.0 {
        return;
    }

    let changed_voice_channel_members = changed_voice_channel.members(&ctx.cache).await.unwrap();

    if changed_voice_channel_members.len() == 1 {
        call.queue().stop();
        call.leave().await.expect("Couldn't leave voice channel");
    }
}
