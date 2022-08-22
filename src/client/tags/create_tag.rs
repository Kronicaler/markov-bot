use regex::Regex;
use serenity::{
    model::prelude::interaction::application_command::{
        ApplicationCommandInteraction, CommandDataOptionValue,
    },
    prelude::Context,
};
use sqlx::{MySql, Pool};

pub async fn create_tag(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    pool: &Pool<MySql>,
) {
    let guild_id = match command.guild_id {
        Some(g) => g,
        None => {
            tag_outside_server_response(command, ctx).await;
            return;
        }
    };

    let (listener, response) = get_listener_and_response(command);

    if !is_tag_valid(response, listener) {
        invalid_tag_response(command, ctx).await;
        return;
    }

    match super::data_access::create_tag(
        listener.to_lowercase().trim().to_owned(),
        response.trim().to_owned(),
        command.user.name.clone(),
        command.user.id.0,
        guild_id.0,
        pool,
    )
    .await
    {
        Ok(tag) => {
            tag_created_response(command, &tag.listener, ctx).await;
        }
        Err(e) => match e {
            super::data_access::CreateTagError::TagWithSameListenerExists => {
                tag_exists_response(command, listener, ctx).await;
            }
        },
    };
}

async fn tag_exists_response(
    command: &ApplicationCommandInteraction,
    listener: &str,
    ctx: &Context,
) {
    command
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| {
                d.content(format!("The tag \"{}\" already exists", listener))
            })
        })
        .await
        .expect("Error creating interaction response");
}

async fn tag_created_response(
    command: &ApplicationCommandInteraction,
    listener: &str,
    ctx: &Context,
) {
    command
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| d.content(format!("Created tag {}", listener)))
        })
        .await
        .expect("Error creating interaction response");
}

async fn invalid_tag_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| d.content("can't add a mention"))
        })
        .await
        .expect("Error creating interaction response");
}

async fn tag_outside_server_response(command: &ApplicationCommandInteraction, ctx: &Context) {
    command
        .create_interaction_response(&ctx.http, |r| {
            r.interaction_response_data(|d| d.content("Can't create a tag outside of a server"))
        })
        .await
        .expect("Error creating interaction response");
}

fn get_listener_and_response(command: &ApplicationCommandInteraction) -> (&String, &String) {
    let listener = command
        .data
        .options
        .get(0)
        .unwrap()
        .options
        .get(0)
        .expect("Expected listener option")
        .resolved
        .as_ref()
        .expect("Expected listener value");
    let response = command
        .data
        .options
        .get(0)
        .unwrap()
        .options
        .get(1)
        .expect("Expected response option")
        .resolved
        .as_ref()
        .expect("Expected response value");

    let listener = match listener {
        CommandDataOptionValue::String(s) => s,
        _ => panic!("Expected listener to be a string"),
    };

    let response = match response {
        CommandDataOptionValue::String(s) => s,
        _ => panic!("Expected listener to be a string"),
    };

    (listener, response)
}

fn is_tag_valid(response: &str, listener: &str) -> bool {
    let user_regex = Regex::new(r"<@!?(\d+)>").expect("Invalid regular expression");
    let role_regex = Regex::new(r"<@&(\d+)>").expect("Invalid regular expression");

    !(user_regex.is_match(response)
        || user_regex.is_match(listener)
        || role_regex.is_match(response)
        || role_regex.is_match(listener)
        || response.contains("@everyone")
        || response.contains("@here"))
}
