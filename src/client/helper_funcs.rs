use std::{
    path::Path,
    process::{Output, Stdio},
    time::Duration,
};

use chrono::Utc;
use file_format::{FileFormat, Kind};
use serenity::{
    all::{
        CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateAttachment,
        CreateInteractionResponseMessage, EditInteractionResponse, Message,
    },
    builder::CreateInteractionResponse,
    model::{
        channel::GuildChannel,
        guild::Guild,
        id::{ChannelId, GuildId},
    },
};
use thiserror::Error;
use tokio::{io::AsyncWriteExt, time::timeout};
use tracing::{Instrument, error, info, info_span};

#[tracing::instrument(skip(ctx))]
pub async fn user_id_command(ctx: &Context, command: &CommandInteraction) {
    let options = &command
        .data
        .options
        .first()
        .expect("Expected user option")
        .value;

    let response = if let CommandDataOptionValue::User(user) = options {
        format!(
            "{}'s id is {}",
            user.to_user(&ctx.http).await.unwrap().name,
            user.get()
        )
    } else {
        "Please provide a valid user".to_owned()
    };

    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(response),
            ),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Couldn't create interaction response");
}

#[tracing::instrument(skip(ctx))]
pub async fn ping_command(ctx: &Context, command: &CommandInteraction) {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content("Pong!"),
            ),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Couldn't create interaction response");
}

/// Will first check the cache for the voice channel and then try the REST API
pub async fn get_guild_channel(
    guild_id: serenity::model::id::GuildId,
    ctx: &Context,
    channel_id: ChannelId,
) -> anyhow::Result<GuildChannel> {
    let guild = guild_id.to_guild_cached(&ctx.cache).map(|g| g.to_owned());

    Ok(match guild {
        Some(guild) => get_guild_channel_from_cache(&guild, channel_id)?,
        None => fetch_guild_channel(guild_id, ctx, channel_id).await?,
    })
}

async fn fetch_guild_channel(
    guild_id: GuildId,
    ctx: &Context,
    channel_id: ChannelId,
) -> anyhow::Result<GuildChannel> {
    Ok(guild_id
        .channels(&ctx.http)
        .await?
        .get(&channel_id)
        .ok_or(GetGuildChannelError::ChannelNotInGuild)?
        .clone())
}

fn get_guild_channel_from_cache(
    guild: &Guild,
    channel_id: ChannelId,
) -> anyhow::Result<GuildChannel> {
    Ok(guild
        .channels
        .get(&channel_id)
        .ok_or(GetGuildChannelError::ChannelNotInGuild)?
        .clone())
}

#[derive(Debug, thiserror::Error)]
pub enum GetGuildChannelError {
    #[error("The requested channel doesn't exist in the guild")]
    ChannelNotInGuild,
}

pub fn get_full_command_name(command: &CommandInteraction) -> String {
    let mut sub_command_group = None;
    let mut sub_command = None;

    for option in &command.data.options {
        match option.kind() {
            CommandOptionType::SubCommand => {
                sub_command_group = Some(&option.name);
            }
            CommandOptionType::SubCommandGroup => {
                sub_command = Some(&option.name);
            }
            _ => {}
        }
    }

    match (sub_command_group, sub_command) {
        (None, None) => command.data.name.to_string(),
        (None, Some(b)) => command.data.name.to_string() + " " + b,
        (Some(a), None) => command.data.name.to_string() + " " + a,
        (Some(a), Some(b)) => command.data.name.to_string() + " " + a + " " + b,
    }
}

#[tracing::instrument(skip(ctx, command, message))]
pub async fn post_file_from_message(
    ctx: &Context,
    command: &CommandInteraction,
    message: &Message,
) {
    let mut max_filesize_mb = 50;
    loop {
        let res = download_file_from_message(message, max_filesize_mb).await;

        let (file_bytes, extension) = match res {
            Ok(file) => file,
            Err(e) => {
                command
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content(e.to_string()),
                    )
                    .instrument(info_span!("Sending message"))
                    .await
                    .expect("Couldn't create interaction response");
                return;
            }
        };

        let res = command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().new_attachment(CreateAttachment::bytes(
                    file_bytes,
                    format!("doki-{}.{}", Utc::now().timestamp() - 1575072000, extension),
                )),
            )
            .instrument(info_span!("Sending message"))
            .await;

        if res.is_err() {
            max_filesize_mb = match max_filesize_mb {
                50 => 10,
                10 => {
                    command
                        .edit_response(
                            &ctx.http,
                            EditInteractionResponse::new().content("Video too large"),
                        )
                        .instrument(info_span!("Sending message"))
                        .await
                        .expect("Couldn't create interaction response");
                    return;
                }
                _ => panic!("unexpected max_filesize_mb"),
            };
            continue;
        } else {
            break;
        }
    }
}

#[derive(Error, Debug)]
pub enum DownloadFileFromMessageError {
    #[error("No link or file found")]
    NoLinkFound,
    #[error("Unsupported link or file")]
    UnsupportedLink,
    #[error("File too large")]
    FileTooLarge,
}

/// Searches the message for a link or attachment
#[tracing::instrument(skip(message))]
pub async fn download_file_from_message(
    message: &Message,
    max_filesize_mb: usize,
) -> Result<(Vec<u8>, String), DownloadFileFromMessageError> {
    let link_regex =
        regex::Regex::new(r#"(?:(?:https?|ftp)://|\b(?:[a-z\d]+\.))(?:(?:[^\s()<>]+|\((?:[^\s()<>]+|(?:\([^\s()<>]+\)))?\))+(?:\((?:[^\s()<>]+|(?:\(?:[^\s()<>]+\)))?\)|[^\s`!()\[\]{};:'".,<>?«»“”‘’]))?"#)
        .expect("Invalid regular expression");

    let Some(query) = link_regex.find(&message.content) else {
        let Some(attachment) = message.attachments.first() else {
            return Err(DownloadFileFromMessageError::NoLinkFound);
        };
        let mut attachment_bytes = attachment
            .download()
            .await
            .map_err(|_| DownloadFileFromMessageError::UnsupportedLink)?;
        let mut extension = Path::new(&attachment.filename)
            .extension()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let file_format = FileFormat::from_bytes(&attachment_bytes);
        if file_format.kind() != Kind::Audio
            && file_format.kind() != Kind::Video
            && file_format.kind() != Kind::Image
            && file_format.kind() != Kind::Other
        {
            error!(
                name = file_format.name(),
                kind = ?file_format.kind(),
                ext = file_format.extension()
            );

            return Err(DownloadFileFromMessageError::UnsupportedLink);
        }

        if file_format.media_type() == "video/mp2t" {
            attachment_bytes = convert_mpegts_to_mp4(attachment_bytes).await.stdout;
            extension = "mp4".to_string();
        }

        return Ok((attachment_bytes, extension));
    };

    let filesize_filter =
        format!("b[filesize<{max_filesize_mb}M]/b[filesize_approx<{max_filesize_mb}M]/b");
    let args = [
        "-f",
        filesize_filter.as_str(),
        "-o",
        "-",
        "--max-filesize",
        &format!("{max_filesize_mb}M"),
        query.as_str(),
    ];
    info!(?args);
    let mut output = tokio::process::Command::new("yt-dlp")
        .args(args)
        .output()
        .instrument(info_span!("waiting on yt-dlp"))
        .await
        .unwrap();

    let stderr_str = str::from_utf8(&output.stderr)
        .unwrap_or_default()
        .to_string();

    if !output.status.success() {
        if stderr_str.contains("Requested format is not available") {
            let formats = tokio::process::Command::new("yt-dlp")
                .args(["--list-formats", query.as_str()])
                .output()
                .instrument(info_span!("waiting on yt-dlp"))
                .await
                .unwrap();

            let formats = String::from_utf8(formats.stdout).unwrap();

            if formats.contains("FILESIZE") {
                error!(stderr_str);
                return Err(DownloadFileFromMessageError::FileTooLarge);
            }

            output = tokio::process::Command::new("yt-dlp")
                .args([
                    "-o",
                    "-",
                    "--max-filesize",
                    &format!("{max_filesize_mb}M"),
                    query.as_str(),
                ])
                .output()
                .instrument(info_span!("waiting on yt-dlp"))
                .await
                .map_err(|_| DownloadFileFromMessageError::FileTooLarge)?;

            if output.stdout.len() > max_filesize_mb * 1_000_000 {
                error!(stderr_str);
                return Err(DownloadFileFromMessageError::FileTooLarge);
            }
        } else {
            error!(stderr_str);
            return Err(DownloadFileFromMessageError::UnsupportedLink);
        }
    }

    let mut file_format = FileFormat::from_bytes(&output.stdout);
    if file_format.kind() != Kind::Audio
        && file_format.kind() != Kind::Video
        && file_format.kind() != Kind::Image
        && file_format.kind() != Kind::Other
    {
        error!(
            name = file_format.name(),
            kind = ?file_format.kind(),
            ext = file_format.extension()
        );

        return Err(DownloadFileFromMessageError::UnsupportedLink);
    }

    if file_format.media_type() == "video/mp2t" {
        output = convert_mpegts_to_mp4(output.stdout).await;
        file_format = FileFormat::from_bytes(&output.stdout);
    }

    Ok((output.stdout, file_format.extension().to_string()))
}

pub async fn convert_mpegts_to_mp4(bytes: Vec<u8>) -> Output {
    let mut ffmpeg = tokio::process::Command::new("ffmpeg")
        .args([
            "-i",
            "-",
            "-c",
            "copy",
            "-bsf:a",
            "aac_adtstoasc",
            "-movflags",
            "+frag_keyframe+empty_moov",
            "-f",
            "mp4",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = ffmpeg.stdin.take().unwrap();
    tokio::spawn(async move {
        stdin.write_all(&bytes).await.unwrap();
    });

    timeout(Duration::from_secs(60), ffmpeg.wait_with_output())
        .await
        .unwrap()
        .unwrap()
}
