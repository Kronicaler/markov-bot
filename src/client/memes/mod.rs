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

use crate::client::memes::dal::{
    create_meme_file, create_new_category_dirs, hash_exists, save_meme_to_file,
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

    let Some(category) = dal::get_category_by_name(tag, pool).await? else {
        bail!("category doesn't exist");
    };

    let server_category = dal::get_server_category(server_id as i64, category.id, pool)
        .await?
        .unwrap_or(dal::MemeServerCategory {
            server_id: server_id as i64,
            category_id: category.id,
            file_id: 1,
        });

    let Some(mut meme_file) = dal::get_file_by_id(server_category.file_id, pool).await? else {
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
    dal::set_server_category(server_id as i64, category.id, meme_file.id + 1, pool).await?;

    Ok((file, file_bytes))
}

fn calculate_hash<T: Hash>(t: &T) -> i64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish() as i64
}

/// - hash bytes and check if it already is in the DB
///     - if it exists in the DB then just add new categories and return
/// - if not then:
/// - if there's a new category make a new directory for it and make a new command for the category
/// - save to folder name of first category
/// - save hash, path and categories to DB
#[tracing::instrument(err, skip(pool))]
pub async fn save_meme(
    name: String,
    bytes: Vec<u8>,
    categories: &Vec<String>,
    pool: &PgPool,
    ctx: &Context,
) -> anyhow::Result<()> {
    let hash = calculate_hash(&bytes);

    // Avoid saving duplicates with hashing
    if hash_exists(hash, pool).await? {
        create_meme_file_categories(categories, hash, pool).await;
        return Ok(());
    }

    create_new_category_dirs(categories).await?;
    create_new_categories(categories, pool).await?;
    save_meme_to_file(&name, &bytes, categories.first().unwrap()).await?;
    create_meme_file(categories.first().unwrap(), &name, hash, pool).await?;

    Ok(())
}

async fn create_new_categories(
    _categories: &[String],
    _pool: &sqlx::Pool<sqlx::Postgres>,
) -> anyhow::Result<()> {
    todo!()
}

async fn create_meme_file_categories(
    _categories: &[String],
    _hash: i64,
    _pool: &sqlx::Pool<sqlx::Postgres>,
) {
    todo!()
}

pub async fn post_meme(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    command.defer(&ctx.http).await.unwrap();

    todo!()
}

pub async fn upload_meme(ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
    command.defer(&ctx.http).await.unwrap();

    let message_id = command.data.target_id.unwrap();

    let message = command
        .data
        .resolved
        .messages
        .get(&message_id.into())
        .unwrap();

    let link_regex =
        regex::Regex::new(r#"(?:(?:https?|ftp)://|\b(?:[a-z\d]+\.))(?:(?:[^\s()<>]+|\((?:[^\s()<>]+|(?:\([^\s()<>]+\)))?\))+(?:\((?:[^\s()<>]+|(?:\(?:[^\s()<>]+\)))?\)|[^\s`!()\[\]{};:'".,<>?«»“”‘’]))?"#)
        .expect("Invalid regular expression");

    let Some(_query) = link_regex.find(&message.content) else {
        command
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("Unsupported or no link found"),
            )
            .instrument(info_span!("Sending message"))
            .await
            .expect("Couldn't create interaction response");

        return Ok(());
    };

    todo!()
}
