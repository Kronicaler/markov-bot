use regex::Regex;
use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CreateInteractionResponseMessage},
    builder::CreateInteractionResponse,
    prelude::Context,
};
use sqlx::{Postgres, Pool};
use tracing::{info_span, Instrument};

#[tracing::instrument(skip(ctx))]
pub async fn create_tag(ctx: &Context, command: &CommandInteraction, pool: &Pool<Postgres>) {
    let Some(guild_id) = command.guild_id else {
        tag_outside_server_response(command, ctx).await;
        return;
    };

    let (listener, response) = get_listener_and_response(command);

    if !is_tag_valid(&response, &listener) {
        invalid_tag_response(command, ctx).await;
        return;
    }

    match super::data_access::create_tag(
        listener.to_lowercase().trim().to_owned(),
        response.trim().to_owned(),
        command.user.name.clone(),
        command.user.id.get() as i64,
        guild_id.get() as i64,
        pool,
    )
    .await
    {
        Ok(tag) => {
            tag_created_response(command, &tag.listener, ctx).await;
        }
        Err(e) => match e {
            super::data_access::CreateTagError::TagWithSameListenerExists => {
                tag_exists_response(command, &listener, ctx).await;
            }
        },
    }
}

async fn tag_exists_response(command: &CommandInteraction, listener: &str, ctx: &Context) {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!("The tag \"{listener}\" already exists")),
            ),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

async fn tag_created_response(command: &CommandInteraction, listener: &str, ctx: &Context) {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(format!("Created tag {listener}")),
            ),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

async fn invalid_tag_response(command: &CommandInteraction, ctx: &Context) {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Tags can't contain mentions or non alphanumeric characters"),
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
                    .content("Can't create a tag outside of a server"),
            ),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Error creating interaction response");
}

fn get_listener_and_response(command: &CommandInteraction) -> (String, String) {
    let (listener, response) = if let CommandDataOptionValue::SubCommand(sub_command) =
        command.data.options.first().unwrap().value.clone()
    {
        (
            sub_command.first().unwrap().value.clone(),
            sub_command.get(1).unwrap().value.clone(),
        )
    } else {
        panic!("The first option should be a SubCommand");
    };

    let CommandDataOptionValue::String(listener) = listener else {
        panic!("Expected listener to be a string")
    };

    let CommandDataOptionValue::String(response) = response else {
        panic!("Expected listener to be a string")
    };

    (listener, response)
}

fn is_tag_valid(response: &str, listener: &str) -> bool {
    let user_regex = Regex::new(r"<@!?(\d+)>").expect("Invalid regular expression");
    let role_regex = Regex::new(r"<@&(\d+)>").expect("Invalid regular expression");
    let alphanumeric_regex = Regex::new(r"[^A-Za-z0-9 ]").expect("Invalid regular expression");

    !(user_regex.is_match(response)
        || user_regex.is_match(listener)
        || role_regex.is_match(response)
        || role_regex.is_match(listener)
        || response.contains("@everyone")
        || response.contains("@here")
        || alphanumeric_regex.is_match(listener))
}
