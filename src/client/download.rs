use std::process::Output;
use std::process::Stdio;
use std::time::Duration;

use anyhow::bail;
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

    process_query(ctx, command, query).await.unwrap();
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

    process_query(ctx, command, query.as_str()).await.unwrap();
}

#[tracing::instrument(skip(ctx, command))]
async fn process_query(
    ctx: Context,
    command: &CommandInteraction,
    query: &str,
) -> anyhow::Result<()> {
    let mut max_filesize_mb: usize = 51;

    loop {
        max_filesize_mb = match max_filesize_mb {
            51 => 50,
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
                bail!("Video too large");
            }
            _ => bail!("unexpected max_filesize_mb"),
        };

        let filesize_filter =
            format!("b[filesize<{max_filesize_mb}M]/b[filesize_approx<{max_filesize_mb}M]");
        let args = ["-f", filesize_filter.as_str(), "-o", "-", query];
        info!(?args);
        let mut output = tokio::process::Command::new("yt-dlp")
            .args(args)
            .output()
            .instrument(info_span!("waiting on yt-dlp"))
            .await
            .unwrap();

        let stderr_str = str::from_utf8(&output.stderr).unwrap_or_default();

        if !output.status.success() {
            error!(stderr_str);

            if stderr_str.contains("Requested format is not available") {
                command
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content("Video too large"),
                    )
                    .instrument(info_span!("Sending message"))
                    .await
                    .expect("Couldn't create interaction response");
                bail!("Video too large");
            } else {
                command
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content("Unsupported link"),
                    )
                    .instrument(info_span!("Sending message"))
                    .await
                    .expect("Couldn't create interaction response");
                bail!("unsupported link");
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

            command
                .edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("Unsupported link"),
                )
                .instrument(info_span!("Sending message"))
                .await
                .expect("Couldn't create interaction response");
            bail!("unsupported link");
        };

        if file_format.media_type() == "video/mp2t" {
            output = convert_mpegts_to_mp4(output.stdout).await;
            file_format = FileFormat::from_bytes(&output.stdout);
        }

        let res = command
            .edit_response(
                &ctx.http,
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
            .await;

        if res.is_err() {
            continue;
        }

        return Ok(());
    }
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
