use super::{
    helper_funcs::{get_voice_channel_of_user, is_bot_in_another_channel},
    Handler, MyAuxMetadata,
};
use reqwest::Client;
use serenity::{
    builder::{CreateEmbed, EditInteractionResponse},
    client::Context,
    model::prelude::{
        interaction::application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
        Colour,
    },
};
use songbird::{
    input::{AuxMetadata, Input, YoutubeDl},
    tracks::TrackQueue,
    TrackEvent,
};
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    time::Duration,
};

///play song from youtube
pub async fn play(ctx: &Context, command: &ApplicationCommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");
    let query = get_query(command);

    command.defer(&ctx.http).await.unwrap();

    let guild = &ctx
        .cache
        .guild(guild_id)
        .expect("unable to fetch guild from the cache")
        .to_owned();

    let voice_channel_id =
        if let Some(voice_channel_id) = get_voice_channel_of_user(guild, command.user.id) {
            voice_channel_id
        } else {
            voice_channel_not_found_response(command, ctx).await;
            return;
        };

    let manager = songbird::get(ctx).await.expect("songbird error").clone();

    let queue = match manager.get(guild_id) {
        Some(e) => Some(e.lock().await.queue().clone()),
        None => None,
    };

    if is_bot_in_another_channel(ctx, guild, command.user.id)
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

    add_track_end_event(&mut call, command, ctx);

    //get source from YouTube
    let source = get_source(query.clone()).await;

    match source {
        SourceType::Video(source) => {
            let mut input: Input = source.into();

            let metadata = input.aux_metadata().await.unwrap();

            let track_handle = call.enqueue_input(input).await;

            let my_metadata = MyAuxMetadata(metadata.clone());

            track_handle
                .typemap()
                .write()
                .await
                .insert::<MyAuxMetadata>(Arc::new(RwLock::new(my_metadata)));

            return_response(&metadata, call.queue(), command, ctx).await;
        }
        SourceType::Playlist(mut sources) => {
            let mut input: Input = sources.pop_front().unwrap().into();

            let metadata = input.aux_metadata().await.unwrap_or_default();

            let track_handle = call.enqueue_input(input).await;

            let my_metadata = MyAuxMetadata(metadata.clone());

            track_handle
                .typemap()
                .write()
                .await
                .insert::<MyAuxMetadata>(Arc::new(RwLock::new(my_metadata)));

            return_response(&metadata, call.queue(), command, ctx).await;

            for source in sources {
                let mut input: Input = source.into();

                let metadata = input.aux_metadata().await.unwrap();

                let track_handle = call.enqueue_input(input).await;

                let my_metadata = MyAuxMetadata(metadata.clone());

                track_handle
                    .typemap()
                    .write()
                    .await
                    .insert::<MyAuxMetadata>(Arc::new(RwLock::new(my_metadata)));
            }
        }
    }
}

fn add_track_end_event(
    call: &mut tokio::sync::MutexGuard<songbird::Call>,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
) {
    if call.queue().is_empty() {
        call.remove_all_global_events();
        call.add_global_event(
            songbird::Event::Track(TrackEvent::End),
            Handler {
                voice_text_channel: command.channel_id,
                guild_id: command.guild_id.unwrap(),
                ctx: ctx.clone(),
            },
        );
    }
}

fn get_query(command: &ApplicationCommandInteraction) -> String {
    let query = command
        .data
        .options
        .iter()
        .find(|opt| opt.name == "query")
        .expect("expected input");

    match query.value.clone() {
        CommandDataOptionValue::String(s) => s,
        _ => panic!("expected a string"),
    }
}

async fn voice_channel_not_found_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new()
                .content("You must be in a voice channel to use this command!"),
        )
        .await
        .expect("Error creating interaction response");
}

async fn voice_channel_not_same_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new()
                .content("You must be in the same voice channel to use this command!"),
        )
        .await
        .expect("Error creating interaction response");
}

enum SourceType {
    Video(YoutubeDl),
    Playlist(VecDeque<YoutubeDl>),
}

async fn get_source(query: String) -> SourceType {
    let link_regex =
    regex::Regex::new(r#"(?:(?:https?|ftp)://|\b(?:[a-z\d]+\.))(?:(?:[^\s()<>]+|\((?:[^\s()<>]+|(?:\([^\s()<>]+\)))?\))+(?:\((?:[^\s()<>]+|(?:\(?:[^\s()<>]+\)))?\)|[^\s`!()\[\]{};:'".,<>?«»“”‘’]))?"#)
    .expect("Invalid regular expression");

    let list_regex = regex::Regex::new(r#"(&list).*|(\?list).*"#).unwrap();
    let playlist_regex = regex::Regex::new(r#"playlist\?list="#).unwrap();

    if link_regex.is_match(&query) {
        if playlist_regex.is_match(&query) {
            let ytdlp_command = std::process::Command::new("yt-dlp")
                .args([&query, "-J", "--flat-playlist"])
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap();

            let playlist_end = std::str::from_utf8(
                &std::process::Command::new("jq")
                    .arg(".entries | length")
                    .stdin(ytdlp_command.stdout.unwrap())
                    .output()
                    .unwrap()
                    .stdout,
            )
            .unwrap()
            .trim()
            .to_string();

            let playlist_end = playlist_end.parse().unwrap();
            let mut songs_in_playlist = VecDeque::default();
            let mut threads = vec![];

            for i in 1..=playlist_end {
                threads.push(tokio::spawn(async move {
                    let client = Client::new();

                    let video_id = std::str::from_utf8(
                        &tokio::process::Command::new("yt-dlp")
                            .args([
                                "yt-dlp",
                                "--get-id",
                                "https://www.youtube.com/playlist?list=PL4D5CEDC7C3A0A193",
                                "-I",
                                &i.to_string(),
                            ])
                            .output()
                            .await
                            .unwrap()
                            .stdout,
                    )
                    .unwrap()
                    .to_string();

                    println!("{video_id}");

                    let song_in_playlist = YoutubeDl::new(
                        client.clone(),
                        format!("https://www.youtube.com/watch?v={}", video_id),
                    );

                    song_in_playlist
                }));
            }

            for t in threads {
                let song = t.await.unwrap();

                songs_in_playlist.push_back(song);
            }

            return SourceType::Playlist(songs_in_playlist);
        }

        // Remove breaking part of url
        let query = list_regex.replace(&query, "").to_string();
        return SourceType::Video(YoutubeDl::new(Client::new(), query));
    }

    let query = format!("ytsearch:{}", query);

    return SourceType::Video(YoutubeDl::new(Client::new(), query));
}

async fn return_response(
    metadata: &AuxMetadata,
    queue: &TrackQueue,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
) {
    let mut embed = create_track_embed(metadata);

    let time = metadata.duration.unwrap_or_else(|| Duration::new(0, 0));

    let mut durations = vec![];

    for track in queue.current_queue() {
        durations.push(
            track
                .typemap()
                .read()
                .await
                .get::<MyAuxMetadata>()
                .unwrap()
                .read()
                .unwrap()
                .0
                .duration
                .unwrap(),
        )
    }

    let time_before_song = durations
        .into_iter()
        .reduce(|a, f| a.checked_add(f).unwrap())
        .and_then(|d| Some(d))
        .unwrap_or_default()
        - time;

    let time_before_song = format!(
        "{}:{:02}",
        time_before_song.as_secs() / 60,
        time_before_song.as_secs() - (time_before_song.as_secs() / 60) * 60
    );

    let content = if queue.len() == 1 {
        "Playing".to_owned()
    } else {
        embed = embed.field("Estimated time until playing: ", time_before_song, true);
        format!("Position in queue: {}", queue.len())
    };

    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new().embed(embed).content(content),
        )
        .await
        .expect("Error creating interaction response");
}

pub fn create_track_embed(metadata: &AuxMetadata) -> CreateEmbed {
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
        .field("Duration: ", duration, true)
        .thumbnail(thumbnail)
        .url(url)
        .clone();

    embed
}
