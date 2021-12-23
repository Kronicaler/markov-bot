use serenity::{
    client::Context,
    model::{
        interactions::application_command::{
            ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
        },
        prelude::Ready,
    },
};

pub fn user_id_command(command: &ApplicationCommandInteraction) -> String {
    let options = command
        .data
        .options
        .get(0)
        .expect("Expected user option")
        .resolved
        .as_ref()
        .expect("Expected user object");
    if let ApplicationCommandInteractionDataOptionValue::User(user, _member) = options {
        format!("{}'s id is {}", user, user.id)
    } else {
        "Please provide a valid user".to_owned()
    }
}

/// Leaves all guilds in which it can't find the bot owner
pub async fn leave_unknown_guilds(ready: &Ready, ctx: &Context) {
    let bot_owner = ctx
        .http
        .get_current_application_info()
        .await
        .expect("couldn't get application info")
        .owner;
    for guild in &ready.guilds {
        let bot_owner_in_guild = guild.id().member(&ctx.http, bot_owner.id).await;

        if let Err(serenity::Error::Http(_)) = bot_owner_in_guild {
            println!("Couldn't find user in guild");
            guild
                .id()
                .leave(&ctx.http)
                .await
                .expect("Couldn't leave guild");
            println!(
                "Left guild {} owned by {}",
                guild
                    .id()
                    .name(&ctx.cache)
                    .await
                    .unwrap_or_else(|| "NO_NAME".to_owned()),
                guild
                    .id()
                    .to_guild_cached(&ctx.cache)
                    .await
                    .expect("Couldn't fetch server from cache")
                    .owner_id
                    .to_user(&ctx.http)
                    .await
                    .expect("Couldn't fetch owner of guild")
                    .tag()
            )
        }
    }
}
