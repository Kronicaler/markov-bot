use std::process::Output;
use std::process::Stdio;
use std::time::Duration;

use chrono::Utc;
use file_format::FileFormat;
use file_format::Kind;
use serenity::all::CreateAttachment;

use serenity::all::EditInteractionResponse;

use serenity::all::ResolvedValue;

use serenity::all::CommandInteraction;

use serenity::client::Context;
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;
use tracing::error;
use tracing::info;
use tracing::info_span;
use tracing::Instrument;

#[tracing::instrument(skip(ctx, command))]
pub async fn download_command(ctx: Context, command: &CommandInteraction) {
    command.defer(&ctx.http).await.unwrap();

    let ResolvedValue::String(query) = command.data.options()[0].value else {
        panic!("unknown command")
    };

    process_query(ctx, command, query).await;
}

#[tracing::instrument(skip(ctx, command))]
pub async fn download_from_message_command(ctx: Context, command: &CommandInteraction) {
    command.defer(&ctx.http).await.unwrap();

    let message_id = command.data.target_id.unwrap();

    let message = command
        .data
        .resolved
        .messages
        .get(&message_id.into())
        .unwrap();

    let link_regex =
        regex::Regex::new(r#"(?:(?:https?|ftp)://|\b(?:[a-z\d]+\.))(?:(?:[^\s()<>]+|\((?:[^\s()<>]+|(?:\([^\s()<>]+\)))?\))+(?:\((?:[^\s()<>]+|(?:\(?:[^\s()<>]+\)))?\)|[^\s`!()\[\]{};:'".,<>?«»“”‘’]))?"#)
        .expect("Invalid regular expression");

    let Some(query) = link_regex.find(&message.content) else {
        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("Unsupported or no link found"),
            )
            .instrument(info_span!("Sending message"))
            .await
            .expect("Couldn't create interaction response");
        return;
    };

    process_query(ctx, command, query.as_str()).await;
}

#[tracing::instrument(skip(ctx, command))]
async fn process_query(ctx: Context, command: &CommandInteraction, query: &str) {
    let mut output = tokio::process::Command::new("yt-dlp")
        .args([
            "--no-hls-use-mpegts",
            "-q",
            "--remux-video",
            "mp4",
            "-o",
            "-",
            query,
        ])
        .output()
        .await
        .unwrap();

    info!("{}", String::from_utf8(output.stderr).unwrap_or_default());

    if output.stdout.len() > 10_000_000 {
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

        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("Unsupported link"),
            )
            .instrument(info_span!("Sending message"))
            .await
            .expect("Couldn't create interaction response");
        return;
    };

    if file_format.media_type() == "video/mp2t" {
        output = convert_mpegts_to_mp4(output.stdout).await;
        file_format = FileFormat::from_bytes(&output.stdout);
    }

    command
        .edit_response(
            ctx.http,
            EditInteractionResponse::new().new_attachment(CreateAttachment::bytes(
                output.stdout,
                format!(
                    "doki-{}.{}",
                    Utc::now().timestamp() - 1575072000,
                    file_format.extension()
                ),
            )),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Couldn't create interaction response");
}

async fn convert_mpegts_to_mp4(bytes: Vec<u8>) -> Output {
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

    return timeout(Duration::from_secs(60), ffmpeg.wait_with_output())
        .await
        .unwrap()
        .unwrap();
}
