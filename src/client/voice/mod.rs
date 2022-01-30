mod play;
pub mod commands;

use serenity::model::id::GuildId;
use serenity::model::prelude::VoiceState;
use serenity::utils::Colour;
use serenity::{
    client::Context, model::interactions::application_command::ApplicationCommandInteraction,
};

/*
 * voice.rs, LsangnaBoi 2022
 * voice channel functionality
 */
pub async fn play(ctx: &Context, command: &ApplicationCommandInteraction) {
    play::play(ctx, command).await;
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
        return;
    }
    let call_lock = call_lock.expect("Couldn't get handler lock");
    let call = call_lock.lock().await;

    if call.queue().is_empty() {
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
