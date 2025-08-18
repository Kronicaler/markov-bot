use chrono::Utc;
use serenity::all::CreateAttachment;

use serenity::all::EditInteractionResponse;

use serenity::all::ResolvedValue;

use serenity::all::CommandInteraction;

use serenity::client::Context;
use tracing::info_span;
use tracing::Instrument;

#[tracing::instrument(skip(ctx))]
pub async fn download_command(ctx: Context, command: &CommandInteraction) {
    command.defer(&ctx.http).await.unwrap();

    let ResolvedValue::String(query) = command.data.options()[0].value else {
        panic!("unknown command")
    };

    let attachment_bytes = tokio::process::Command::new("yt-dlp")
        .args(["yt-dlp", "-o", "-", query])
        .output()
        .await
        .unwrap()
        .stdout;

    let Some(file_type) = infer::get(&attachment_bytes) else {
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

    command
        .edit_response(
            ctx.http,
            EditInteractionResponse::new().new_attachment(CreateAttachment::bytes(
                attachment_bytes,
                format!(
                    "doki-{}.{}",
                    Utc::now().timestamp() - 1575072000,
                    file_type.extension()
                ),
            )),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Couldn't create interaction response");
}

#[tracing::instrument(skip(ctx))]
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

    let attachment_bytes = tokio::process::Command::new("yt-dlp")
        .args(["yt-dlp", "-o", "-", query.as_str()])
        .output()
        .await
        .unwrap()
        .stdout;

    let Some(file_type) = infer::get(&attachment_bytes) else {
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

    command
        .edit_response(
            ctx.http,
            EditInteractionResponse::new().new_attachment(CreateAttachment::bytes(
                attachment_bytes,
                format!("doki-{}.{}", Utc::now().timestamp() - 1575072000, file_type.extension()),
            )),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Couldn't create interaction response");
}
