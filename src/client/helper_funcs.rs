use serenity::{
    client::Context,
    model::{
        interactions::application_command::{
            ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
        },
        prelude::Ready,
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
    let response =
        if let ApplicationCommandInteractionDataOptionValue::User(user, _member) = options {
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
        let guild_id = guild_status.id();

        let bot_owner_in_guild = guild_id.member(&ctx.http, bot_owner.id).await;

        if let Err(serenity::Error::Http(_)) = bot_owner_in_guild {
            println!("Couldn't find bot owner in guild");

            guild_id
                .leave(&ctx.http)
                .await
                .expect("Couldn't leave guild");

            let guild_name = guild_id
                .name(&ctx.cache)
                .await
                .unwrap_or_else(|| "NO_NAME".to_owned());

            let guild_owner = guild_id
                .to_guild_cached(&ctx.cache)
                .await
                .expect("Couldn't fetch guild from cache")
                .owner_id
                .to_user(&ctx.http)
                .await
                .expect("Couldn't fetch owner of guild")
                .tag();

            println!("Left guild {guild_name} owned by {guild_owner}",)
        }
    }
}
