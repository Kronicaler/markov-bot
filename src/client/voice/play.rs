use serenity::{
    builder::CreateEmbed,
    client::Context,
    model::interactions::application_command::{
        ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
    },
    utils::Colour,
};
use songbird::{
    create_player,
    input::{Input, Metadata, Restartable},
    tracks::TrackQueue,
};
use std::time::Duration;

use super::helper_funcs::*;

///play song from youtube
pub async fn play(ctx: &Context, command: &ApplicationCommandInteraction) {
    //get the guild ID, cache, and query
    let guild_id = command.guild_id.expect("Couldn't get guild ID");
    let cache = &ctx.cache;
    let query = get_query(command);

    command.defer(&ctx.http).await.unwrap();

    let guild = cache
        .guild(guild_id)
        .await
        .expect("unable to fetch guild from the cache");

    // Get voice channel_id
    let voice_channel_id =
        if let Some(voice_channel_id) = get_voice_channel_of_user(&guild, command.user.id) {
            voice_channel_id
        } else {
            voice_channel_not_found_response(command, ctx).await;
            return;
        };

    //create manager
    let manager = songbird::get(ctx).await.expect("songbird error").clone();

    let queue = match manager.get(guild_id) {
        Some(e) => Some(e.lock().await.queue().clone()),
        None => None,
    };

    if is_bot_in_another_channel(ctx, &guild, command.user.id).await
        && queue.is_some()
        && !queue.expect("Should never fail").is_empty()
    {
        voice_channel_not_same_response(command, ctx).await;
        return;
    }

    //join voice channel
    let (call_lock, success) = manager.join(guild_id, voice_channel_id).await;
    if success.is_err() {
        voice_channel_not_found_response(command, ctx).await;
        return;
    }
    let mut call = call_lock.lock().await;

    //get source from YouTube
    let source = get_source(query.to_owned(), command, ctx)
        .await
        .expect("Couldn't get source");

    // Return interaction response
    let input: Input = source.into();
    return_response(&input.metadata, call.queue(), command, ctx).await;

    //add to queue
    let (mut audio, _) = create_player(input);
    audio.set_volume(0.5);
    call.enqueue(audio);
}

fn get_query(command: &ApplicationCommandInteraction) -> &String {
    let query = command
        .data
        .options
        .iter()
        .find(|opt| opt.name == "query")
        .expect("expected input");

    match query.resolved.as_ref().unwrap() {
        ApplicationCommandInteractionDataOptionValue::String(s) => s,
        _ => panic!("expected a string"),
    }
}

async fn voice_channel_not_found_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .edit_original_interaction_response(&ctx.http, |r| {
            r.content("You must be in a voice channel to use this command!")
        })
        .await
        .expect("Error creating interaction response");
}

async fn voice_channel_not_same_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .edit_original_interaction_response(&ctx.http, |r| {
            r.content("You must be in the same voice channel to use this command!")
        })
        .await
        .expect("Error creating interaction response");
}

async fn get_source(
    query: String,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
) -> Result<Restartable, Box<dyn std::error::Error>> {
    let link_regex =
    regex::Regex::new(r#"(?:(?:https?|ftp)://|\b(?:[a-z\d]+\.))(?:(?:[^\s()<>]+|\((?:[^\s()<>]+|(?:\([^\s()<>]+\)))?\))+(?:\((?:[^\s()<>]+|(?:\(?:[^\s()<>]+\)))?\)|[^\s`!()\[\]{};:'".,<>?«»“”‘’]))?"#)
    .expect("Invalid regular expression");

    if link_regex.is_match(&query) {
        match Restartable::ytdl(query, false).await {
            Ok(source) => return Ok(source),
            Err(why) => {
                println!("Err starting source: {:?}", why);
                command
                    .edit_original_interaction_response(&ctx.http, |r| {
                        r.content("Coulnd't find the video on Youtube")
                    })
                    .await
                    .expect("Error creating interaction response");
                return Err(Box::new(why));
            }
        }
    }

    match Restartable::ytdl_search(query, false).await {
        Ok(source) => Ok(source),
        Err(why) => {
            println!("Err starting source: {:?}", why);
            command
                .edit_original_interaction_response(&ctx.http, |r| {
                    r.content("Coulnd't find the video on Youtube")
                })
                .await
                .expect("Error creating interaction response");
            Err(Box::new(why))
        }
    }
}

async fn return_response(
    metadata: &Metadata,
    queue: &TrackQueue,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
) {
    let title = metadata.title.clone().unwrap_or_default();
    let channel = metadata.channel.clone().unwrap_or_default();
    let thumbnail = metadata.thumbnail.clone().unwrap_or_default();
    let url = metadata.source_url.clone().unwrap_or_default();
    let time = metadata.duration.unwrap_or_else(|| Duration::new(0, 0));
    let minutes = time.as_secs() / 60;
    let seconds = time.as_secs() - minutes * 60;
    let duration = format!("{}:{:02}", minutes, seconds);
    let colour = Colour::from_rgb(149, 8, 2);

    let embed = CreateEmbed::default()
        .title(title)
        .colour(colour)
        .description(channel)
        .field("duration: ", duration, false)
        .thumbnail(thumbnail)
        .url(url)
        .clone();

    let content = if queue.is_empty() {
        "Playing"
    } else {
        "Queued up"
    };

    command
        .edit_original_interaction_response(&ctx.http, |r| r.add_embed(embed).content(content))
        .await
        .expect("Error creating interaction response");
}
