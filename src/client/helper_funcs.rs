use serenity::{
    client::Context,
    model::{
        channel::GuildChannel,
        guild::Guild,
        id::{ChannelId, GuildId},
        prelude::{
            command::CommandOptionType,
            interaction::application_command::{
                ApplicationCommandInteraction, CommandDataOptionValue,
            },
        },
    },
};

pub async fn user_id_command(ctx: Context, command: &ApplicationCommandInteraction) {
    let options = command
        .data
        .options
        .get(0)
        .expect("Expected user option")
        .resolved
        .as_ref()
        .expect("Expected user object");
    let response = if let CommandDataOptionValue::User(user, _member) = options {
        format!("{}'s id is {}", user, user.id)
    } else {
        "Please provide a valid user".to_owned()
    };

    command
        .create_interaction_response(ctx.http, |r| {
            r.interaction_response_data(|d| d.content(response))
        })
        .await
        .expect("Couldnt create interaction response");
}

pub async fn ping_command(ctx: Context, command: &ApplicationCommandInteraction) {
    command
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| d.content("Pong!"))
        })
        .await
        .expect("Couldn't create interaction response");
}

/// Will first check the cache for the voice channel and then try the REST API
pub async fn get_guild_channel(
    guild_id: serenity::model::id::GuildId,
    ctx: &Context,
    channel_id: ChannelId,
) -> anyhow::Result<GuildChannel> {
    let changed_voice_channel = match guild_id.to_guild_cached(&ctx.cache) {
        Some(guild) => get_guild_channel_from_cache(&guild, channel_id)?,
        None => fetch_guild_channel(guild_id, ctx, channel_id).await?,
    };
    Ok(changed_voice_channel)
}

async fn fetch_guild_channel(
    guild_id: GuildId,
    ctx: &Context,
    channel_id: ChannelId,
) -> Result<GuildChannel, anyhow::Error> {
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
) -> Result<GuildChannel, anyhow::Error> {
    Ok(guild
        .channels
        .get(&channel_id)
        .ok_or(GetGuildChannelError::ChannelNotInGuild)?
        .clone()
        .guild()
        .ok_or(GetGuildChannelError::NotGuildChannel)?)
}

#[derive(Debug, thiserror::Error)]
pub enum GetGuildChannelError {
    #[error("The requested channel doesn't exist in the guild")]
    ChannelNotInGuild,
    #[error("The requested channel isn't a GuildChannel")]
    NotGuildChannel,
}

pub fn get_full_command_name(command: &ApplicationCommandInteraction) -> String {
    let mut sub_command_group = None;
    let mut sub_command = None;

    for option in &command.data.options {
        match option.kind {
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
