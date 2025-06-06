use serenity::{
    all::{
        CommandDataOptionValue, CommandInteraction, CommandOptionType, CreateAttachment,
        CreateInteractionResponseMessage, EditInteractionResponse, ResolvedValue,
    },
    builder::CreateInteractionResponse,
    client::Context,
    model::{
        channel::GuildChannel,
        guild::Guild,
        id::{ChannelId, GuildId},
    },
};
use tracing::{info_span, Instrument};
use uuid::Uuid;

#[tracing::instrument(skip(ctx))]
pub async fn user_id_command(ctx: Context, command: &CommandInteraction) {
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
            ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(response),
            ),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Couldn't create interaction response");
}

#[tracing::instrument(skip(ctx))]
pub async fn ping_command(ctx: Context, command: &CommandInteraction) {
    command
        .create_response(
            ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content("Pong!"),
            ),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Couldn't create interaction response");
}

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
                format!("dokibot-{}.{}", Uuid::new_v4(), file_type.extension()),
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
                format!("dokibot-{}.{}", Uuid::new_v4(), file_type.extension()),
            )),
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
            _ => continue,
        };
    }

    match (sub_command_group, sub_command) {
        (None, None) => command.data.name.clone(),
        (None, Some(b)) => command.data.name.clone() + " " + b,
        (Some(a), None) => command.data.name.clone() + " " + a,
        (Some(a), Some(b)) => command.data.name.clone() + " " + a + " " + b,
    }
}
