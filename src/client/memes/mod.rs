// purpose: to send videos and images to a server from a folder in alphabetical or random order when prompted
//
// behavior:
// - we have a folder "my_folder" with 3 videos: A.mp4, B.mp4 and C.mp4
// - a command is created for the folder with the same name of the folder
// - the command is executed in server X and Y
// - the "my_folder" command is executed in server X and A.mp4 is posted
// - the "my_folder" command is executed in server X and B.mp4 is posted
// - the "my_folder" command is executed in server Y and A.mp4 is posted
// - the "my_folder" command is executed in server Y and B.mp4 is posted
// - if all the files have been sent from the folder then it loops again from the beginning

pub mod commands;
mod dal;

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    time::Duration,
};

use itertools::Itertools;
use serenity::all::{
    CommandInteraction, Context, CreateAttachment, CreateQuickModal, EditInteractionResponse,
    QuickModal,
};
use sqlx::{PgConnection, PgPool};
use tracing::{Instrument, info, info_span};

use crate::client::{
    get_option_from_command::GetOptionFromCommand,
    helper_funcs::download_file_from_message,
    memes::dal::{
        MemeServerCategory, create_meme_file, create_meme_file_categories, create_new_categories,
        create_new_category_dirs, get_file_by_hash, get_meme_file_count_by_folder,
        save_meme_to_file,
    },
};

pub const MEMES_FOLDER: &str = "./data/memes";

fn calculate_hash<T: Hash>(t: &T) -> i64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish() as i64
}

/// - hash bytes and check if it already is in the DB
///     - if it exists in the DB then just add new categories in the db and return
/// - if not then:
/// - if there's a new category make a new directory for it and make a new command for the category
/// - save to folder name of first category
/// - save hash, path and categories to DB
#[tracing::instrument(err, skip(pool, bytes))]
pub async fn save_meme(
    bytes: &[u8],
    extension: &str,
    categories: &[String],
    pool: &PgPool,
) -> anyhow::Result<()> {
    let extension = &extension.to_lowercase();
    let categories = &categories.iter().map(|e| e.to_lowercase()).collect_vec();

    let hash = calculate_hash(&bytes);

    let mut tx = pool.begin().await?;

    let meme_file = get_file_by_hash(hash, &mut tx).await?;

    // Avoid saving duplicates with hashing
    if let Some(meme_file) = meme_file {
        create_new_categories(categories, &mut tx).await?;
        create_meme_file_categories(categories, meme_file.id, &mut tx).await?;
        return Ok(());
    }

    let folder = categories.first().unwrap();
    let number = get_meme_file_count_by_folder(folder, &mut tx).await? + 1;
    let name = format!("{folder}_{number}");

    create_new_category_dirs(&vec![folder.clone()]).await?;
    save_meme_to_file(&name, extension, bytes, folder).await?;

    create_new_categories(categories, &mut tx).await?;
    let meme_file_id = create_meme_file(folder, &name, extension, hash, &mut tx).await?;
    create_meme_file_categories(categories, meme_file_id, &mut tx).await?;

    tx.commit().await?;

    Ok(())
}

#[tracing::instrument(err, skip(ctx, command, pool))]
pub async fn post_meme_command(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &PgPool,
) -> anyhow::Result<()> {
    let category = command.data.get_string("tag").to_lowercase();
    let is_ordered = command
        .data
        .get_optional_bool("ordered")
        .unwrap_or_default();
    let is_ephemeral = command
        .data
        .get_optional_bool("ephemeral")
        .unwrap_or_default();

    if is_ephemeral {
        command.defer_ephemeral(&ctx.http).await.unwrap();
    } else {
        command.defer(&ctx.http).await.unwrap();
    }

    let mut tx = pool.begin().await?;

    if is_ordered {
        post_ordered_meme(ctx, command, &category, &mut tx).await?;
    } else {
        post_random_meme(ctx, command, category, &mut tx).await?;
    }

    tx.commit().await?;

    Ok(())
}

#[tracing::instrument(err, skip(ctx, command, conn))]
async fn post_random_meme(
    ctx: &Context,
    command: &CommandInteraction,
    category: String,
    conn: &mut PgConnection,
) -> Result<(), anyhow::Error> {
    let mfc = dal::get_random_meme_file_category_by_category(&category, conn).await?;
    let Some(mfc) = mfc else {
        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("Tag doesn't exist"),
            )
            .await?;
        return Ok(());
    };
    post_meme(ctx, command, conn, mfc.file_id).await?;
    Ok(())
}

#[tracing::instrument(err, skip(ctx, command, conn))]
async fn post_ordered_meme(
    ctx: &Context,
    command: &CommandInteraction,
    category: &String,
    conn: &mut PgConnection,
) -> Result<(), anyhow::Error> {
    let category = dal::get_categories_by_name(&[category.clone()], conn)
        .await?
        .pop();

    let Some(category) = category else {
        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("Tag doesn't exist"),
            )
            .await?;
        return Ok(());
    };

    let server_id = command
        .guild_id
        .map_or_else(|| command.channel_id.get(), serenity::all::GuildId::get)
        as i64;
    let mut server_category = dal::get_server_category(server_id, category.id, conn)
        .await?
        .unwrap_or(MemeServerCategory {
            category_id: category.id,
            file_id: 1,
            server_id,
        });

    if dal::get_file_by_id(server_category.file_id, conn)
        .await?
        .is_none()
    {
        let oldest_mfc = dal::get_oldest_meme_file_category(category.id, conn)
            .await?
            .unwrap();

        server_category.file_id = oldest_mfc.file_id;

        dal::set_server_category(
            server_category.server_id,
            oldest_mfc.category_id,
            oldest_mfc.file_id,
            conn,
        )
        .await?;
    }

    post_meme(ctx, command, conn, server_category.file_id).await?;

    let next_mfc =
        dal::get_next_meme_file_category(category.id, server_category.file_id, conn).await?;

    let Some(next_mfc) = next_mfc else {
        info!("reached the oldest meme file category, resetting to beginning");
        let oldest_mfc = dal::get_oldest_meme_file_category(category.id, conn)
            .await?
            .unwrap();

        dal::set_server_category(
            server_category.server_id,
            server_category.category_id,
            oldest_mfc.file_id,
            conn,
        )
        .await?;

        return Ok(());
    };

    dal::set_server_category(
        server_category.server_id,
        server_category.category_id,
        next_mfc.file_id,
        conn,
    )
    .await?;

    Ok(())
}

#[tracing::instrument(err, skip(ctx, command, tx))]
async fn post_meme(
    ctx: &Context,
    command: &CommandInteraction,
    tx: &mut PgConnection,
    file_id: i32,
) -> Result<(), anyhow::Error> {
    let meme_file = dal::get_file_by_id(file_id, tx).await?;
    let Some(meme_file) = meme_file else {
        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("Tag doesn't exist"),
            )
            .await?;
        return Ok(());
    };
    let file_bytes = dal::read_file(&meme_file)?;
    command
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().new_attachment(CreateAttachment::bytes(
                file_bytes,
                format!("{}.{}", meme_file.name, meme_file.extension),
            )),
        )
        .await?;
    Ok(())
}

#[tracing::instrument(err, skip(ctx, command, pool))]
pub async fn upload_meme(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &PgPool,
) -> anyhow::Result<()> {
    let modal_response = command
        .quick_modal(
            ctx,
            CreateQuickModal::new("Upload meme")
                .timeout(Duration::from_mins(10))
                .text("Input the tags of the meme you selected. Make sure the tags are separated by spaces. For example for a cute cat video you'd input the tags ``cat cute``")
                .short_field("Tags"),
        )
        .await?;

    let Some(modal_response) = modal_response else {
        return Ok(());
    };

    modal_response
        .interaction
        .defer_ephemeral(&ctx.http)
        .await?;

    let message_id = command.data.target_id.unwrap();

    let message = command
        .data
        .resolved
        .messages
        .get(&message_id.into())
        .unwrap();

    let (file_bytes, extension) = download_file_from_message(message, 50).await?;

    let categories = modal_response.inputs[0]
        .split(' ')
        .map(|s| s.to_lowercase().clone())
        .collect_vec();

    if categories.is_empty() {
        info!("no categories provided");

        modal_response
            .interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("No tags provided"),
            )
            .instrument(info_span!("Sending message"))
            .await
            .expect("Couldn't create interaction response");

        return Ok(());
    }

    save_meme(&file_bytes, &extension, &categories, pool).await?;

    modal_response
        .interaction
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content("Saved meme. I can now post it when someone runs ``/meme post`` with one of the tags you provided!"),
        )
        .instrument(info_span!("Sending message"))
        .await
        .expect("Couldn't create interaction response");

    Ok(())
}
