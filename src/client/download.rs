use std::str::FromStr;

use crate::client::helper_funcs::post_file_from_message;
use serenity::{
    all::{CommandInteraction, Context, Message, ResolvedValue},
    small_fixed_array::FixedString,
};

#[tracing::instrument(skip(ctx, command))]
pub async fn download_command(ctx: &Context, command: &CommandInteraction) {
    command.defer(&ctx.http).await.unwrap();

    let ResolvedValue::String(query) = command.data.options()[0].value else {
        panic!("unknown command")
    };

    let mut x = Message::default();
    x.content = FixedString::from_str(query).unwrap();

    post_file_from_message(ctx, command, &x).await;
}

#[tracing::instrument(skip(ctx, command))]
pub async fn download_from_message_command(ctx: &Context, command: &CommandInteraction) {
    command.defer(&ctx.http).await.unwrap();

    let message_id = command.data.target_id.unwrap();

    let message = command
        .data
        .resolved
        .messages
        .get(&message_id.into())
        .unwrap();

    post_file_from_message(ctx, command, message).await;
}
