// purpose: to send videos and images to a server from a folder in alphabetical order when prompted
//
// behaviour:
// - we have a folder "my_folder" with 3 videos: A.mp4, B.mp4 and C.mp4
// - a command is created for the folder with the same name of the folder
// - the command is executed in server X and Y
// - the "my_folder" command is executed in server X and A.mp4 is posted
// - the "my_folder" command is executed in server X and B.mp4 is posted
// - the "my_folder" command is executed in server Y and A.mp4 is posted
// - the "my_folder" command is executed in server Y and B.mp4 is posted
// - if all the memes have been sent from the folder then it loops

pub mod commands;
mod dal;
pub mod model;

use std::fs::{self, DirEntry};

use anyhow::bail;
use itertools::Itertools;
use sqlx::MySqlPool;

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

    let mut files = fs::read_dir(format!("./data/memes/{folder_name}"))?
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

    // find file by index

    // if index is out of bounds reset it to 0
    if files.len() < index as usize {
        index = 0;
    }

    let file = files.swap_remove(index as usize);

    let file_bytes = fs::read(file.path())?;

    // update folder_index
    dal::set_server_folder_index(server_id, folder_name, index + 1, pool).await?;

    Ok((file, file_bytes))
}

#[tracing::instrument(ret)]
pub fn get_meme_folders() -> Vec<DirEntry> {
    let Ok(folders) = fs::read_dir("./data/memes") else {
        return vec![];
    };

    let folders = folders
        .filter_map(|f| f.ok())
        .filter(|f| f.file_type().is_ok_and(|f| f.is_dir()))
        .collect_vec();

    folders
}
