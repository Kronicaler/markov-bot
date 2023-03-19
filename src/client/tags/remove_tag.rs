use serenity::{
    builder::{CreateInteractionResponse, CreateInteractionResponseData},
    model::prelude::interaction::application_command::{
        ApplicationCommandInteraction, CommandDataOptionValue,
    },
    prelude::Context,
};
use sqlx::{MySql, Pool};

use super::data_access;

pub async fn remove_tag(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    pool: &Pool<MySql>,
) {
    let listener = get_listener(command);

    let tag = data_access::get_tag_by_listener(
        &listener,
        command
            .guild_id
            .expect("This command can't be called outside guilds")
            .get(),
        pool,
    )
    .await;

    match tag {
        Some(tag) => {
            data_access::delete_tag(tag.id, pool).await;

            println!(
                "{} removed tag {} in server {}",
                command.user.name, tag.listener, tag.server_id
            );
            tag_removed_response(command, &tag.listener, ctx).await;
        }
        None => {
            tag_not_found_response(command, &listener, ctx).await;
        }
    }
}

async fn tag_not_found_response(
    command: &ApplicationCommandInteraction,
    listener: &str,
    ctx: &Context,
) {
    command
        .create_interaction_response(
            &ctx.http,
            CreateInteractionResponse::new().interaction_response_data(
                CreateInteractionResponseData::new()
                    .content(format!("Couldn't find the tag {listener}")),
            ),
        )
        .await
        .expect("Error creating interaction response");
}

async fn tag_removed_response(
    command: &ApplicationCommandInteraction,
    listener: &str,
    ctx: &Context,
) {
    command
        .create_interaction_response(
            &ctx.http,
            CreateInteractionResponse::new().interaction_response_data(
                CreateInteractionResponseData::new().content(format!("Removed the tag {listener}")),
            ),
        )
        .await
        .expect("Error creating interaction response");
}

fn get_listener(command: &ApplicationCommandInteraction) -> String {
    let listener = if let CommandDataOptionValue::SubCommand(sub_command) =
        command.data.options.get(0).unwrap().value.clone()
    {
        sub_command.get(0).unwrap().value.clone()
    } else {
        panic!("Expected the first option to be a subcommand");
    };

    match listener {
        CommandDataOptionValue::String(l) => l,
        _ => panic!("Listener was expected to be a string"),
    }
}
