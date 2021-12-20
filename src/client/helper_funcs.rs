use serenity::{model::{interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
}, prelude::Ready}, client::Context};

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

pub async fn leave_unknown_guilds(ready: &Ready, ctx: &Context) {
    for guild in &ready.guilds {
        match guild
            .id()
            .member(
                &ctx.http,
                ctx.http
                    .get_current_application_info()
                    .await
                    .expect("couldn't get application info")
                    .owner
                    .id,
            )
            .await
        {
            Err(_) => guild
                .id()
                .leave(&ctx.http)
                .await
                .expect("couldn't leave guild"),
            _ => {}
        };
    }
}
