// purpose: to send videos and images to a server from a folder in alphabetical or random order when prompted
//
// behaviour:
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

use std::fs::{self, DirEntry};

use anyhow::bail;
use itertools::Itertools;
use rand::Rng;
use sqlx::MySqlPool;

const MEMES_FOLDER: &'static str = "./data/memes";
const RANDOM_MEMES_FOLDER: &'static str = "./data/random_memes";

#[tracing::instrument(err, skip(pool))]
pub async fn read_meme(
    server_id: u64,
    folder_name: &str,
    pool: &MySqlPool,
) -> anyhow::Result<(DirEntry, Vec<u8>)> {
    // fetch file index from db for this folder and server

    let mut index = dal::get_server_folder_index(server_id, folder_name, pool)
        .await?
        .map(|i| i.file_index)
        .unwrap_or(0);

    // read dir and sort by name

    let mut files = fs::read_dir(format!("{MEMES_FOLDER}/{folder_name}"))?
        .filter_map(|f| f.ok())
        .sorted_by(|a, b| {
            alphanumeric_sort::compare_str(
                a.file_name().to_string_lossy().to_string(),
                b.file_name().to_string_lossy().to_string(),
            )
        })
        .collect_vec();

    if files.is_empty() {
        bail!("no files in folder");
    }

    // if index is out of bounds set it to 0
    if files.len() < index as usize {
        index = 0;
    }

    // find file by index
    let file = files.swap_remove(index as usize);

    let file_bytes = fs::read(file.path())?;

    // update folder_index
    dal::set_server_folder_index(server_id, folder_name, index + 1, pool).await?;

    Ok((file, file_bytes))
}

#[tracing::instrument(err)]
pub async fn read_random_meme(
    folder_name: &str,
) -> anyhow::Result<(DirEntry, Vec<u8>)> {
    let mut files = fs::read_dir(format!("{RANDOM_MEMES_FOLDER}/{folder_name}"))?
        .filter_map(|f| f.ok())
        .collect_vec();

    if files.is_empty() {
        bail!("no files in folder");
    }

    let index = rand::thread_rng().gen_range(0..files.len());

    let file = files.swap_remove(index as usize);

    let file_bytes = fs::read(file.path())?;

    Ok((file, file_bytes))
}

#[tracing::instrument(ret)]
pub fn get_meme_folders() -> Vec<DirEntry> {
    let Ok(folders) = fs::read_dir(MEMES_FOLDER) else {
        return vec![];
    };

    let folders = folders
        .filter_map(|f| f.ok())
        .filter(|f| f.file_type().is_ok_and(|f| f.is_dir()))
        .collect_vec();

    folders
}

#[tracing::instrument(ret)]
pub fn get_random_meme_folders() -> Vec<DirEntry> {
    let Ok(folders) = fs::read_dir(RANDOM_MEMES_FOLDER) else {
        return vec![];
    };

    let folders = folders
        .filter_map(|f| f.ok())
        .filter(|f| f.file_type().is_ok_and(|f| f.is_dir()))
        .collect_vec();

    folders
}
