use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CreateInteractionResponseMessage, UserId},
    builder::CreateInteractionResponse,
    prelude::Context,
};
use sqlx::{Pool, Postgres};
use tracing::{Instrument, info_span};

use crate::client::tags::data_access::{
    create_tag_banned_user, delete_tag_banned_user, get_tag_banned_user,
};

#[tracing::instrument(skip(ctx))]
pub async fn ban_user_from_editing_tags(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &Pool<Postgres>,
) {
    let Some(guild_id) = command.guild_id else {
        tag_outside_server_response(command, ctx).await;
        return;
    };

    if !command
        .user
        .member
        .as_ref()
        .unwrap()
        .permissions
        .unwrap()
        .moderate_members()
    {
        admins_only_response(command, ctx).await;
    }

    let user = get_user(command);
    let user_id = user.get() as i64;
    let server_id = guild_id.get() as i64;

    let tag_banned_user = get_tag_banned_user(user_id, server_id, pool).await;

    if tag_banned_user.is_none() {
        create_tag_banned_user(user_id, server_id, pool).await;
        tag_ban_created_response(command, ctx).await;
    } else {
        delete_tag_banned_user(user_id, server_id, pool).await;
        tag_ban_deleted_response(command, ctx).await;
    }
}

async fn tag_ban_created_response(command: &CommandInteraction, ctx: &Context) {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!("Banned user from editing tags")),
            ),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

async fn tag_ban_deleted_response(command: &CommandInteraction, ctx: &Context) {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!("Unbanned user from editing tags")),
            ),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

async fn tag_outside_server_response(command: &CommandInteraction, ctx: &Context) {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Can't run this command outside of a server"),
            ),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

async fn admins_only_response(command: &CommandInteraction, ctx: &Context) {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Only moderators can run this command"),
            ),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

fn get_user(command: &CommandInteraction) -> UserId {
    let user = if let CommandDataOptionValue::SubCommand(sub_command) =
        command.data.options.first().unwrap().value.clone()
    {
        sub_command.first().unwrap().value.clone()
    } else {
        panic!("The first option should be a SubCommand");
    };

    let CommandDataOptionValue::User(user) = user else {
        panic!("Expected user to be a user")
    };

    user
}
