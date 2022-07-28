use serenity::{
    client::Context,
    model::{prelude::{
        interaction::application_command::{ApplicationCommandInteraction, CommandDataOptionValue},
        Ready,
    }, id::{ChannelId, GuildId}, channel::GuildChannel, guild::Guild},
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

/// Leaves all guilds in which it can't find the bot owner
pub async fn leave_unknown_guilds(ready: &Ready, ctx: &Context) {
    let bot_owner = ctx
        .http
        .get_current_application_info()
        .await
        .expect("Couldn't get application info")
        .owner;
    for guild_status in &ready.guilds {
        let guild_id = guild_status.id;

        let bot_owner_in_guild = guild_id.member(&ctx.http, bot_owner.id).await;

        if let Err(serenity::Error::Http(_)) = bot_owner_in_guild {
            println!("Couldn't find bot owner in guild");

            guild_id
                .leave(&ctx.http)
                .await
                .expect("Couldn't leave guild");

            let guild_name = guild_id
                .name(&ctx.cache)
                .unwrap_or_else(|| "NO_NAME".to_owned());

            let guild_owner = guild_id
                .to_guild_cached(&ctx.cache)
                .expect("Couldn't fetch guild from cache")
                .owner_id
                .to_user(&ctx.http)
                .await
                .expect("Couldn't fetch owner of guild")
                .tag();

            println!("Left guild {guild_name} owned by {guild_owner}",);
        }
    }
}

/// Will first check the cache for the voice channel and then try the REST API
pub async fn get_guild_channel(
    guild_id: serenity::model::id::GuildId,
    ctx: &Context,
    channel_id: ChannelId,
) -> anyhow::Result<GuildChannel> {
    let changed_voice_channel = match guild_id.to_guild_cached(&ctx.cache) {
        Some(guild) => get_guild_channel_from_cache(guild, channel_id)?,
        None => fetch_guild_channel(guild_id, ctx, channel_id).await?,
    };
    Ok(changed_voice_channel)
}

async fn fetch_guild_channel(guild_id: GuildId, ctx: &Context, channel_id: ChannelId) -> Result<GuildChannel, anyhow::Error> {
    Ok(guild_id
        .channels(&ctx.http)
        .await?
        .get(&channel_id)
        .ok_or_else(|| GetGuildChannelError::ChannelNotInGuild)?
        .clone())
}

fn get_guild_channel_from_cache(guild: Guild, channel_id: ChannelId) -> Result<GuildChannel, anyhow::Error> {
    Ok(guild
        .channels
        .get(&channel_id)
        .ok_or_else(|| GetGuildChannelError::ChannelNotInGuild)?
        .clone()
        .guild()
        .ok_or_else(|| GetGuildChannelError::NotGuildChannel)?
        .clone())
}

#[derive(Debug, thiserror::Error)]
pub enum GetGuildChannelError {
    #[error("The requested channel doesn't exist in the guild")]
    ChannelNotInGuild,
    #[error("The requested channel isn't a GuildChannel")]
    NotGuildChannel,
}