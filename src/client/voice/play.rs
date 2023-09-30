use crate::client::{
    helper_funcs::get_guild_channel,
    voice::{create_bring_to_front_button, create_play_now_button, model::get_voice_messages_lock},
};

use super::{
    create_skip_button,
    helper_funcs::{
        get_voice_channel_of_user, is_bot_in_another_voice_channel, voice_channel_not_same_response,
    },
    model::get_queue_data_lock,
    queue::update_queue_message::update_queue_message,
    MyAuxMetadata, PeriodicHandler, TrackStartHandler,
};
use futures::future::join_all;
use reqwest::Client;
use serenity::{
    builder::{CreateActionRow, CreateComponents, CreateEmbed, EditInteractionResponse},
    client::Context,
    model::prelude::{
        interaction::application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
        Colour, GuildId,
    },
    prelude::{Mutex, RwLock},
};
use songbird::{
    input::{AuxMetadata, Input, YoutubeDl},
    tracks::TrackQueue,
    TrackEvent,
};
use std::{collections::VecDeque, ops::ControlFlow, sync::Arc, time::Duration};
use tracing::{info, info_span, warn, Instrument};

///play song from youtube
#[tracing::instrument(skip(ctx))]
pub async fn play(ctx: &Context, command: &ApplicationCommandInteraction) {
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
    let (call_lock, success) = manager
        .join(guild_id, voice_channel_id)
        .instrument(info_span!("Joining channel"))
        .await;

    if success.is_err() {
        voice_channel_not_found_response(command, ctx).await;
        return;
    }

    {
        let mut call = call_lock.lock().await;

        add_track_start_event(&mut call, command, ctx);
    }
    //get source from YouTube
    let source = get_source(query.clone()).await;

    match source {
        SourceType::Video(source) => {
            handle_video(source, command, ctx, &call_lock).await;
        }
        SourceType::Playlist(sources) => {
            handle_playlist(sources, command, ctx, call_lock).await;
        }
    }
}

#[tracing::instrument(skip(ctx, call_lock))]
async fn handle_video(
    source: YoutubeDl,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
    call_lock: &Arc<Mutex<songbird::Call>>,
) -> ControlFlow<()> {
    let mut input: Input = source.into();

    let Ok(metadata) = input.aux_metadata().await else {
        invalid_link_response(command, ctx).await;
        return ControlFlow::Break(());
    };

    let mut call = call_lock.lock().await;
    let track_handle = call.enqueue_input(input).await;
    let my_metadata = MyAuxMetadata(metadata.clone());

    track_handle
        .typemap()
        .write()
        .await
        .insert::<MyAuxMetadata>(Arc::new(RwLock::new(my_metadata)));

    return_response(&metadata, call.queue(), command, ctx).await;
    ControlFlow::Continue(())
}

#[tracing::instrument(skip(sources,ctx,call_lock), fields(sources.length=sources.len()))]
async fn handle_playlist(
    mut sources: VecDeque<YoutubeDl>,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
    call_lock: Arc<Mutex<songbird::Call>>,
) {
    if sources.is_empty() {
        invalid_link_response(command, ctx).await;
        return;
    }

    {
        async {
            let source = sources.pop_front().unwrap();
            let mut input: Input = source.into();
            let metadata = input.aux_metadata().await.unwrap_or_default();
            let mut call = call_lock.lock().await;

            let track_handle = call.enqueue_input(input).await;

            let my_metadata = MyAuxMetadata(metadata.clone());

            track_handle
                .typemap()
                .write()
                .await
                .insert::<MyAuxMetadata>(Arc::new(RwLock::new(my_metadata)));

            return_response(&metadata, call.queue(), command, ctx).await;
        }
        .instrument(info_span!("Play first song"))
        .await;
    }

    fill_queue(sources, call_lock, ctx, command.guild_id.unwrap()).await;
}

#[tracing::instrument(skip(sources,call_lock,ctx), fields(sources.length=sources.len()))]
async fn fill_queue(
    sources: VecDeque<YoutubeDl>,
    call_lock: Arc<Mutex<songbird::Call>>,
    ctx: &Context,
    guild_id: GuildId,
) {
    let inputs: VecDeque<Input> = sources.into_iter().map(Into::into).collect();

    let queue_data_lock = get_queue_data_lock(&ctx.data).await;
    {
        let mut queue_data = queue_data_lock.write().await;
        queue_data.filling_queue.insert(guild_id, true);
    }

    let length = inputs.len();

    let mut futures: Vec<_> = vec![];
    for (i, mut input) in inputs.into_iter().enumerate() {
        let call_lock = call_lock.clone();
        let queue_data_lock = queue_data_lock.clone();
        {
            let mut call = call_lock.lock().await;

            let Some(current_channel) = call.current_channel() else {
                info!("Returning early due to not being connected to any channel");
                return;
            };

            let voice_channel = get_guild_channel(guild_id, ctx, current_channel.0.into())
                .await
                .unwrap();

            if voice_channel.members(&ctx.cache).unwrap().len() == 1 {
                info!("Returning early due to empty channel");

                call.queue().stop();
                call.remove_all_global_events();
                call.leave().await.expect("Couldn't leave voice channel");

                return;
            }
        }

        let x = async move {
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

        futures.push(tokio::spawn(x));

        if i % 10 == 0 {
            let task_results = join_all(futures)
                .await
                .into_iter()
                .filter(std::result::Result::is_ok)
                .filter_map(std::result::Result::unwrap);

            let mut call = call_lock
                .lock()
                .instrument(info_span!("Waiting for call lock"))
                .await;

            for (metadata, input) in task_results {
                let queue_filling_stopped = !*queue_data_lock
                    .read()
                    .await
                    .filling_queue
                    .get(&guild_id)
                    .unwrap();

                if call.current_channel().is_none() || queue_filling_stopped {
                    return;
                }

                let my_metadata = MyAuxMetadata(metadata);

                let track_handle = call.enqueue_input(input).await;

                track_handle
                    .typemap()
                    .write()
                    .await
                    .insert::<MyAuxMetadata>(Arc::new(RwLock::new(my_metadata)));
            }

            update_queue_message(ctx, guild_id).await;

            futures = vec![];
        }
    }
}

async fn invalid_link_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new().content("Invalid link"),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

fn add_track_start_event(
    call: &mut tokio::sync::MutexGuard<songbird::Call>,
    command: &ApplicationCommandInteraction,
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
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

enum SourceType {
    Video(YoutubeDl),
    Playlist(VecDeque<YoutubeDl>),
}

#[tracing::instrument]
async fn get_source(query: String) -> SourceType {
    let link_regex =
    regex::Regex::new(r#"(?:(?:https?|ftp)://|\b(?:[a-z\d]+\.))(?:(?:[^\s()<>]+|\((?:[^\s()<>]+|(?:\([^\s()<>]+\)))?\))+(?:\((?:[^\s()<>]+|(?:\(?:[^\s()<>]+\)))?\)|[^\s`!()\[\]{};:'".,<>?«»“”‘’]))?"#)
    .expect("Invalid regular expression");

    let list_regex = regex::Regex::new(r"(&list).*|(\?list).*").unwrap();
    let playlist_regex = regex::Regex::new(r"playlist\?list=").unwrap();

    if link_regex.is_match(&query) {
        if playlist_regex.is_match(&query) {
            let mut songs_in_playlist = VecDeque::default();
            let client = Client::new();

            std::str::from_utf8(
                &tokio::process::Command::new("yt-dlp")
                    .args(["yt-dlp", "--flat-playlist", "--get-id", &query])
                    .output()
                    .await
                    .unwrap()
                    .stdout,
            )
            .unwrap()
            .to_string()
            .split('\n')
            .filter(|f| !f.is_empty())
            .for_each(|id| {
                let song = YoutubeDl::new(
                    client.clone(),
                    format!("https://www.youtube.com/watch?v={id}"),
                );

                songs_in_playlist.push_back(song);
            });

            return SourceType::Playlist(songs_in_playlist);
        }

        // Remove breaking part of url
        let query = list_regex.replace(&query, "").to_string();
        return SourceType::Video(YoutubeDl::new(Client::new(), query));
    }

    let query = format!("ytsearch:{query}");

    SourceType::Video(YoutubeDl::new(Client::new(), query))
}

async fn return_response(
    metadata: &AuxMetadata,
    queue: &TrackQueue,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
) {
    let mut embed = create_track_embed(metadata);

    let time = metadata.duration.unwrap_or_else(|| Duration::new(0, 0));

    let durations = get_queue_durations(queue).await;

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

    let mut components = CreateComponents::new().add_action_row(CreateActionRow::new());
    let content = if queue.len() == 1 {
        let mut action_row = components.0.pop().unwrap();

        action_row = action_row.add_button(create_skip_button());

        components = components.set_action_row(action_row);

        "Playing".to_owned()
    } else {
        embed = embed.field("Estimated time until playing: ", time_before_song, true);

        let mut action_row = components.0.pop().unwrap();

        action_row = action_row
            .add_button(create_bring_to_front_button())
            .add_button(create_play_now_button());

        components = components.set_action_row(action_row);

        format!("Position in queue: {}", queue.len())
    };

    let message = command
        .edit_original_interaction_response(
            &ctx.http,
            EditInteractionResponse::new()
                .embed(embed)
                .components(components)
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

async fn get_queue_durations(queue: &TrackQueue) -> Vec<Duration> {
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
                .await
                .0
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
