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
pub mod model;

use std::{
    fs::{self, DirEntry},
    hash::{DefaultHasher, Hash, Hasher},
};

use anyhow::bail;
use itertools::Itertools;
use serenity::all::{CommandInteraction, Context, EditInteractionResponse};
use sqlx::PgPool;
use tracing::{Instrument, info_span};

use crate::client::{
    helper_funcs::download_file_from_message,
    memes::dal::{
        create_meme_file, create_meme_file_categories, create_new_categories,
        create_new_category_dirs, get_file_by_hash, get_meme_file_count_by_folder,
        save_meme_to_file,
    },
};

pub const MEMES_FOLDER: &str = "./data/memes";

#[tracing::instrument(err, skip(pool))]
pub async fn read_meme(
    server_id: u64,
    tag: &str,
    ordered: bool,
    pool: &PgPool,
) -> anyhow::Result<(DirEntry, Vec<u8>)> {
    // fetch file index from db for this folder and server

    let mut tx = pool.begin().await?;

    let Some(category) = dal::get_categories_by_name(&vec![tag.to_string()], &mut tx)
        .await?
        .pop()
    else {
        bail!("category doesn't exist");
    };

    let server_category = dal::get_server_category(server_id as i64, category.id, &mut tx)
        .await?
        .unwrap_or(dal::MemeServerCategory {
            server_id: server_id as i64,
            category_id: category.id,
            file_id: 1,
        });

    let Some(mut meme_file) = dal::get_file_by_id(server_category.file_id, &mut tx).await? else {
        bail!("no files for category exist")
    };

    // read dir and sort by name

    let mut files = fs::read_dir(format!("{MEMES_FOLDER}/{}", meme_file.folder))?
        .filter_map(std::result::Result::ok)
        .sorted_by(|a, b| {
            alphanumeric_sort::compare_str(
                a.file_name().to_string_lossy(),
                b.file_name().to_string_lossy(),
            )
        })
        .collect_vec();

    if files.is_empty() {
        bail!("no files in folder");
    }

    // if index is out of bounds set it to 0
    if files.len() < meme_file.id as usize {
        meme_file.id = 1;
    }

    // find file by index
    let file = files.swap_remove(meme_file.id as usize);

    let file_bytes = fs::read(file.path())?;

    // update folder_index
    dal::set_server_category(server_id as i64, category.id, meme_file.id + 1, &mut tx).await?;

    Ok((file, file_bytes))
}

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
    name: String,
    bytes: Vec<u8>,
    categories: &Vec<String>,
    pool: &PgPool,
) -> anyhow::Result<()> {
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
    let number = get_meme_file_count_by_folder(&folder, &mut tx).await? + 1;
    let name = format!("{folder}_{number}");

    create_new_category_dirs(categories).await?;
    save_meme_to_file(&name, &bytes, &folder).await?;

    create_new_categories(categories, &mut tx).await?;
    create_meme_file(&folder, &name, hash, &mut tx).await?;

    Ok(())
}

pub async fn post_meme(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &PgPool,
) -> anyhow::Result<()> {
    command.defer(&ctx.http).await.unwrap();

    todo!()
}

#[tracing::instrument(err, skip(ctx, command, pool))]
pub async fn upload_meme(
    ctx: &Context,
    command: &CommandInteraction,
    pool: &PgPool,
) -> anyhow::Result<()> {
    command.defer_ephemeral(&ctx.http).await.unwrap();

    let message_id = command.data.target_id.unwrap();

    let message = command
        .data
        .resolved
        .messages
        .get(&message_id.into())
        .unwrap();

    let (file_bytes, _) = download_file_from_message(message, 50).await?;

    let categories = vec!["TestCategory".to_string()];

    save_meme("TestName".to_string(), file_bytes, &categories, pool).await?;

    command
        .edit_response(&ctx.http, EditInteractionResponse::new().content("Saved meme"))
        .instrument(info_span!("Sending message"))
        .await
        .expect("Couldn't create interaction response");

    // TODO: show modal with a text field for tags
    todo!()
}
