use serenity::{client::Context, framework::standard::{CommandResult, macros::command}, model::channel::Message};



#[command]
pub async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!").await?;

    Ok(())
}