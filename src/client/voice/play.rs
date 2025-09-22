use crate::client::{
    helper_funcs::get_guild_channel,
    voice::{create_bring_to_front_button, create_play_now_button, model::get_voice_messages_lock},
};

use super::{
    create_shuffle_button, create_skip_button,
    helper_funcs::{
        get_voice_channel_of_user, is_bot_in_another_voice_channel, voice_channel_not_same_response,
    },
    model::get_queue_data_lock,
    queue::update_queue_message::update_queue_message,
    MyAuxMetadata, PeriodicHandler, TrackStartHandler,
};
use futures::{future::join_all, StreamExt};
use infer::MatcherType;
use itertools::Itertools;
use rand::seq::SliceRandom;
use reqwest::Client;
use rspotify::{
    http::HttpError,
    model::{AlbumId, Country, Market, PlaylistId},
    prelude::BaseClient,
    ClientCredsSpotify, ClientError, Credentials,
};
use serenity::{
    all::{Colour, CommandDataOptionValue, CommandInteraction, GuildId},
    builder::{CreateActionRow, CreateEmbed, EditInteractionResponse},
    client::Context,
    prelude::Mutex,
};
use songbird::{
    input::{AuxMetadata, Input, YoutubeDl},
    tracks::{Track, TrackQueue},
    TrackEvent,
};
use std::{collections::VecDeque, sync::Arc, time::Duration};
use tokio::time::timeout;
use tracing::{error, info, info_span, warn, Instrument};
use url::Url;

///play song from youtube
#[tracing::instrument(skip(ctx))]
pub async fn play(ctx: &Context, command: &CommandInteraction) {
    let guild_id = command.guild_id.expect("Couldn't get guild ID");
    let query = get_query(command);

    command.defer(&ctx.http).await.unwrap();

    let guild = &ctx
        .cache
        .guild(guild_id)
        .expect("unable to fetch guild from the cache")
        .to_owned();

    let Some(voice_channel_id) = get_voice_channel_of_user(guild, command.user.id) else {
        voice_channel_not_found_response(command, ctx).await;
        return;
    };

    let manager = songbird::get(ctx).await.expect("songbird error").clone();

    let queue = match manager.get(guild_id) {
        Some(e) => Some(e.lock().await.queue().clone()),
        None => None,
    };

    if is_bot_in_another_voice_channel(ctx, guild, command.user.id)
        && queue.is_some()
        && !queue.expect("Should never fail").is_empty()
    {
        voice_channel_not_same_response(command, ctx).await;
        return;
    }

    //join voice channel
    let Ok(call_lock) = manager
        .join(guild_id, voice_channel_id)
        .instrument(info_span!("Joining channel"))
        .await
    else {
        voice_channel_not_found_response(command, ctx).await;
        return;
    };

    {
        let mut call = timeout(Duration::from_secs(30), call_lock.lock())
            .await
            .unwrap();

        add_track_start_event(&mut call, command, ctx);
    }
    //get source from YouTube
    let source = get_source(query).await;

    match source {
        Some(SourceType::Video(input, metadata)) => {
            handle_video(input, metadata, command, ctx, &call_lock).await;
        }
        Some(SourceType::Playlist(inputs)) => {
            handle_playlist(inputs, command, ctx, call_lock).await;
        }
        None => {
            info!("no sourcetype");
            invalid_link_response(command, ctx).await;
        }
    }
}

#[tracing::instrument(skip(input, ctx, call_lock))]
pub async fn handle_video(
    input: Input,
    metadata: AuxMetadata,
    command: &CommandInteraction,
    ctx: &Context,
    call_lock: &Arc<Mutex<songbird::Call>>,
) {
    let mut call = timeout(Duration::from_secs(30), call_lock.lock())
        .await
        .unwrap();
    let my_metadata = MyAuxMetadata {
        aux_metadata: metadata.clone(),
        queued_by: command.user.name.clone(),
    };
    let track = Track::new_with_data(input, Arc::new(my_metadata));
    call.enqueue(track).await;

    return_response(&metadata, call.queue(), command, ctx, false).await;
}

#[tracing::instrument(skip(inputs, command, ctx, call_lock), fields(inputs.length=inputs.len()))]
async fn handle_playlist(
    mut inputs: VecDeque<Input>,
    command: &CommandInteraction,
    ctx: &Context,
    call_lock: Arc<Mutex<songbird::Call>>,
) {
    if inputs.is_empty() {
        info!("no songs in playlist");
        invalid_link_response(command, ctx).await;
        return;
    }

    {
        async {
            let mut input = inputs.pop_front().unwrap();
            let metadata = input.aux_metadata().await.unwrap_or_default();
            let mut call = timeout(Duration::from_secs(30), call_lock.lock())
                .await
                .unwrap();

            let my_metadata = MyAuxMetadata {
                aux_metadata: metadata.clone(),
                queued_by: command.user.name.clone(),
            };
            let track = Track::new_with_data(input, Arc::new(my_metadata));
            call.enqueue(track).await;

            return_response(&metadata, call.queue(), command, ctx, true).await;
        }
        .instrument(info_span!("Play first song"))
        .await;
    }

    fill_queue(
        inputs,
        call_lock,
        ctx,
        command.guild_id.unwrap(),
        command.user.name.clone(),
    )
    .await;
}

#[tracing::instrument(skip(inputs, call_lock, ctx), fields(inputs.length=inputs.len()))]
async fn fill_queue(
    mut inputs: VecDeque<Input>,
    call_lock: Arc<Mutex<songbird::Call>>,
    ctx: &Context,
    guild_id: GuildId,
    queued_by: String,
) {
    let queue_data_lock = get_queue_data_lock(&ctx.data).await;

    queue_data_lock
        .write()
        .await
        .filling_queue
        .insert(guild_id, true);

    let length = inputs.len();

    let mut fetch_aux_metadata_futures: Vec<_> = vec![];
    for i in 0..(inputs.len() - 1) {
        let shuffle_queue = queue_data_lock
            .read()
            .await
            .shuffle_queue
            .get(&guild_id)
            .cloned()
            .unwrap_or(false);

        if shuffle_queue {
            inputs.make_contiguous().shuffle(&mut rand::thread_rng());

            queue_data_lock
                .write()
                .await
                .shuffle_queue
                .insert(guild_id.clone(), false);
        }

        let mut input = inputs.pop_front().unwrap();

        let call_lock = call_lock.clone();
        let queue_data_lock = queue_data_lock.clone();

        let call = timeout(Duration::from_secs(30), call_lock.lock())
            .await
            .unwrap();

        let Some(current_channel) = call.current_channel() else {
            info!("Returning early due to not being connected to any channel");
            return;
        };

        drop(call);

        let voice_channel = get_guild_channel(guild_id, ctx, current_channel.0.into())
            .await
            .unwrap();

        if voice_channel.members(&ctx.cache).unwrap().len() == 1 {
            info!("Returning early due to empty channel");

            let mut call = timeout(Duration::from_secs(30), call_lock.lock())
                .await
                .unwrap();

            call.queue().stop();
            call.remove_all_global_events();
            call.leave().await.expect("Couldn't leave voice channel");

            return;
        }

        let fetch_aux_metadata = async move {
            match input
                .aux_metadata()
                .instrument(info_span!("Fetching metadata"))
                .await
            {
                Ok(e) => Some((e, input)),
                Err(e) => {
                    warn!("Error when fetching playlist song metadata: {}", e);
                    None
                }
            }
        }
        .instrument(info_span!(
            "Add song to queue",
            playlist.position = i,
            playlist.length = length
        ));

        fetch_aux_metadata_futures.push(tokio::spawn(fetch_aux_metadata));

        if fetch_aux_metadata_futures.len() >= 10 || i == length - 1 {
            let task_results = join_all(fetch_aux_metadata_futures)
                .instrument(info_span!("Fetching metadatas"))
                .await
                .into_iter()
                .filter_map(|f| {
                    match f {
                        Ok(f) => return f,
                        Err(e) => error!("{:?}", e),
                    }
                    None
                });

            for (metadata, input) in task_results {
                let queue_filling_stopped = !queue_data_lock
                    .read()
                    .await
                    .filling_queue
                    .get(&guild_id)
                    .unwrap();

                if call_lock.lock().await.current_channel().is_none() || queue_filling_stopped {
                    return;
                }

                let my_metadata = MyAuxMetadata {
                    aux_metadata: metadata,
                    queued_by: queued_by.clone(),
                };
                let track = Track::new_with_data(input, Arc::new(my_metadata));
                call_lock.lock().await.enqueue(track).await;
            }

            let ctx = ctx.clone();
            tokio::spawn(
                async move {
                    update_queue_message(&ctx, guild_id, call_lock).await;
                }
                .instrument(info_span!("Updating queue message")),
            );

            fetch_aux_metadata_futures = vec![];
        }
    }
}

async fn invalid_link_response(command: &CommandInteraction, ctx: &Context) {
    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content("Invalid link"),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

pub fn add_track_start_event(
    call: &mut tokio::sync::MutexGuard<songbird::Call>,
    command: &CommandInteraction,
    ctx: &Context,
) {
    if call.queue().is_empty() {
        call.remove_all_global_events();
        call.add_global_event(
            songbird::Event::Track(TrackEvent::Play),
            TrackStartHandler {
                voice_text_channel: command.channel_id,
                guild_id: command.guild_id.unwrap(),
                ctx: ctx.clone(),
            },
        );

        call.add_global_event(
            songbird::Event::Periodic(Duration::from_secs(60), None),
            PeriodicHandler {
                guild_id: command.guild_id.unwrap(),
                ctx: ctx.clone(),
            },
        );
    }
}

fn get_query(command: &CommandInteraction) -> String {
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

pub async fn voice_channel_not_found_response(command: &CommandInteraction, ctx: &Context) {
    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new()
                .content("You must be in a voice channel to use this command!"),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

enum SourceType {
    Video(Input, AuxMetadata),
    Playlist(VecDeque<Input>),
}

#[tracing::instrument]
async fn get_source(query: String) -> Option<SourceType> {
    let link_regex =
    regex::Regex::new(r#"(?:(?:https?|ftp)://|\b(?:[a-z\d]+\.))(?:(?:[^\s()<>]+|\((?:[^\s()<>]+|(?:\([^\s()<>]+\)))?\))+(?:\((?:[^\s()<>]+|(?:\(?:[^\s()<>]+\)))?\)|[^\s`!()\[\]{};:'".,<>?«»“”‘’]))?"#)
    .expect("Invalid regular expression");
    let spot_regex = regex::Regex::new(r"spotify.com").unwrap();
    let spot_playlist_regex = regex::Regex::new(r"spotify.com/playlist").unwrap();
    let spot_album_regex = regex::Regex::new(r"spotify.com/album").unwrap();
    let yt_regex = regex::Regex::new(r"^((?:https?:)?\/\/)?((?:www|m)\.)?((?:youtube(?:-nocookie)?\.com|youtu.be))(\/(?:[\w\-]+\?v=|embed\/|live\/|v\/)?)([\w\-]+)(\S+)?$").unwrap();
    let yt_list_regex = regex::Regex::new(r"(&list).*|(\?list).*").unwrap();
    let yt_playlist_regex = regex::Regex::new(r"youtube.*playlist\?list=").unwrap();

    if link_regex.is_match(&query) && yt_regex.is_match(&query) {
        if yt_playlist_regex.is_match(&query) {
            return Some(get_yt_playlist_inputs(&query).await);
        }

        // Remove possible breaking part of url
        let query = yt_list_regex.replace(&query, "").to_string();
        let mut input: Input = YoutubeDl::new(Client::new(), query).into();
        let metadata = input.aux_metadata().await.unwrap();
        return Some(SourceType::Video(input, metadata));
    }

    if link_regex.is_match(&query) && spot_regex.is_match(&query) {
        let creds = Credentials::from_env().unwrap();
        let spotify = ClientCredsSpotify::new(creds);
        spotify.request_token().await.unwrap();

        if spot_playlist_regex.is_match(&query) {
            return Some(get_spotify_playlist_inputs(&query, &spotify).await);
        }

        if spot_album_regex.is_match(&query) {
            return Some(get_spotify_album_inputs(&query, spotify).await);
        }
    }

    if link_regex.is_match(&query) {
        try_get_link_input(&query).await?;
    }

    let query = format!("ytsearch:{query}");

    let mut input: Input = YoutubeDl::new(Client::new(), query).into();
    let metadata = input.aux_metadata().await.unwrap();
    Some(SourceType::Video(input, metadata))
}

async fn try_get_link_input(query: &str) -> Option<SourceType> {
    let video_stream_bytes = tokio::process::Command::new("yt-dlp")
        .args(["yt-dlp", "-o", "-", query])
        .output()
        .await
        .unwrap()
        .stdout;
    let file_type = infer::get(&video_stream_bytes)?;

    if file_type.matcher_type() != MatcherType::Audio
        && file_type.matcher_type() != MatcherType::Video
    {
        return None;
    }

    let mut input: Input = video_stream_bytes.into();
    let metadata = input.aux_metadata().await.unwrap_or_else(|_| AuxMetadata {
        source_url: Some(query.to_string()),
        track: Some(query.to_string()),
        title: Some(query.to_string()),
        ..Default::default()
    });

    Some(SourceType::Video(input, metadata))
}

async fn get_spotify_album_inputs(query: &str, spotify: ClientCredsSpotify) -> SourceType {
    let url = Url::parse(query).unwrap();
    let path_segments = url.path_segments().unwrap().collect_vec();
    let album_id = path_segments.get(1).unwrap();
    let album_id = AlbumId::from_id(
        album_id
            .chars()
            .take_while(|c| *c != '?')
            .join("")
            .trim()
            .to_string(),
    )
    .unwrap();
    let mut album_tracks = spotify.album_track(album_id, Some(Market::Country(Country::Croatia)));
    let mut tracks = VecDeque::new();
    let client = Client::new();
    while let Some(track) = album_tracks.next().await {
        let track = match track {
            Ok(track) => track,
            Err(err) => {
                error!("{:?}", err);

                if let ClientError::Http(err) = err {
                    if let HttpError::StatusCode(err) = *err {
                        error!("{:?}", err.text().await);
                    }
                }
                continue;
            }
        };

        let song_name = track.name;
        let artists = track.artists.into_iter().map(|a| a.name).join(" ");
        let query = format!("ytsearch:{song_name} {artists}");
        let input: Input = YoutubeDl::new(client.clone(), query).into();
        tracks.push_back(input);
    }

    SourceType::Playlist(tracks)
}

async fn get_spotify_playlist_inputs(query: &str, spotify: &ClientCredsSpotify) -> SourceType {
    let url = Url::parse(query).unwrap();
    let path_segments = url.path_segments().unwrap().collect_vec();
    let playlist_id = path_segments.get(1).unwrap();
    let playlist_id = PlaylistId::from_id(
        playlist_id
            .chars()
            .take_while(|c| *c != '?')
            .join("")
            .trim()
            .to_string(),
    )
    .unwrap();

    let mut playlist_items =
        spotify.playlist_items(playlist_id, None, Some(Market::Country(Country::Croatia)));

    let mut tracks = VecDeque::new();
    let client = Client::new();

    while let Some(item) = playlist_items.next().await {
        let item = match item {
            Ok(item) => item,
            Err(err) => {
                error!("{:?}", err);

                if let ClientError::Http(err) = err {
                    if let HttpError::StatusCode(err) = *err {
                        error!("{:?}", err.text().await);
                    }
                }
                continue;
            }
        };

        let track = match item.track.unwrap() {
            rspotify::model::PlayableItem::Track(full_track) => full_track,
            rspotify::model::PlayableItem::Episode(_) => {
                panic!("episodes arent supported")
            }
        };

        let song_name = track.name;
        let artists = track.artists.into_iter().map(|a| a.name).join(" ");
        let query = format!("ytsearch:{song_name} {artists}");
        let input: Input = YoutubeDl::new(client.clone(), query).into();
        tracks.push_back(input);
    }

    SourceType::Playlist(tracks)
}

async fn get_yt_playlist_inputs(query: &str) -> SourceType {
    let client = Client::new();
    let songs_in_playlist = std::str::from_utf8(
        &tokio::process::Command::new("yt-dlp")
            .args(["yt-dlp", "--flat-playlist", "--get-id", query])
            .output()
            .await
            .unwrap()
            .stdout,
    )
    .unwrap()
    .split('\n')
    .filter(|f| !f.is_empty())
    .map(|id| {
        YoutubeDl::new(
            client.clone(),
            format!("https://www.youtube.com/watch?v={id}"),
        )
        .into()
    })
    .collect();

    SourceType::Playlist(songs_in_playlist)
}

async fn return_response(
    metadata: &AuxMetadata,
    queue: &TrackQueue,
    command: &CommandInteraction,
    ctx: &Context,
    is_playlist: bool,
) {
    let mut embed = create_track_embed(metadata);

    let time = metadata.duration.unwrap_or_else(|| Duration::new(0, 0));

    let durations = get_queue_durations(queue);

    let time_before_song = durations
        .into_iter()
        .reduce(|a, f| a.checked_add(f).unwrap())
        .unwrap_or_default()
        - time;

    let time_before_song = format!(
        "{}:{:02}",
        time_before_song.as_secs() / 60,
        time_before_song.as_secs() - (time_before_song.as_secs() / 60) * 60
    );

    let mut buttons = vec![];
    let content = if queue.len() == 1 {
        buttons.push(create_skip_button());

        "Playing".to_owned()
    } else {
        embed = embed.field("Estimated time until playing: ", time_before_song, true);

        buttons.push(create_bring_to_front_button());
        buttons.push(create_play_now_button());

        format!("Position in queue: {}", queue.len())
    };
    if is_playlist {
        buttons.push(create_shuffle_button());
    }
    let action_row = CreateActionRow::Buttons(buttons);

    let message = command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new()
                .embed(embed)
                .components(vec![action_row])
                .content(content),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");

    let voice_messages_lock = get_voice_messages_lock(&ctx.data).await;
    let mut voice_messages = voice_messages_lock.write().await;

    if message.content.contains("Playing") {
        voice_messages
            .last_now_playing
            .insert(command.guild_id.unwrap(), message);
    } else if message.content.contains("Position in queue") {
        voice_messages
            .last_position_in_queue
            .insert(command.guild_id.unwrap(), message);
    }
}

fn get_queue_durations(queue: &TrackQueue) -> Vec<Duration> {
    let mut durations = vec![];

    for track in queue.current_queue() {
        durations.push(
            track
                .data::<MyAuxMetadata>()
                .aux_metadata
                .duration
                .unwrap_or_else(|| Duration::from_secs(0)),
        );
    }

    durations
}

pub fn create_track_embed(metadata: &AuxMetadata) -> CreateEmbed {
    let title = metadata.title.clone().unwrap_or_default();
    let channel = metadata.channel.clone().unwrap_or_default();
    let thumbnail = metadata.thumbnail.clone().unwrap_or_default();
    let url = metadata.source_url.clone().unwrap_or_default();
    let time = metadata.duration.unwrap_or_else(|| Duration::new(0, 0));
    let minutes = time.as_secs() / 60;
    let seconds = time.as_secs() - minutes * 60;
    let duration = format!("{minutes}:{seconds:02}");
    let colour = Colour::from_rgb(149, 8, 2);

    CreateEmbed::default()
        .title(title)
        .colour(colour)
        .description(channel)
        .field("Duration: ", duration, true)
        .thumbnail(thumbnail)
        .url(url)
}
