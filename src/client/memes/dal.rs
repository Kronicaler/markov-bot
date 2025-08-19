use std::fs::{self, DirEntry};

use itertools::Itertools;
use sqlx::MySqlPool;

use crate::client::memes::{MEMES_FOLDER, RANDOM_MEMES_FOLDER};

#[derive(Debug)]
pub struct ServerFolderIndex {
    #[allow(dead_code)]
    pub server_id: u64,
    #[allow(dead_code)]
    pub folder_name: String,
    pub file_index: u32,
}

pub struct FileHash {
    hash: u64,
    path: String,
}

#[tracing::instrument(err, skip(pool))]
pub async fn get_server_folder_index(
    server_id: u64,
    folder_name: &str,
    pool: &MySqlPool,
) -> anyhow::Result<Option<ServerFolderIndex>> {
    Ok(sqlx::query_as!(
        ServerFolderIndex,
        r#"
            SELECT * FROM server_folder_indexes
            WHERE folder_name = ? AND server_id = ?
            "#,
        folder_name,
        server_id
    )
    .fetch_optional(pool)
    .await?)
}

#[tracing::instrument(err, skip(pool))]
pub async fn set_server_folder_index(
    server_id: u64,
    folder_name: &str,
    file_index: u32,
    pool: &MySqlPool,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
		REPLACE INTO server_folder_indexes ( server_id, folder_name, file_index )
		VALUES ( ?, ?, ? )
		"#,
        server_id,
        folder_name,
        file_index
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn save_meme_hash(path: &str, hash: u64, categories: &Vec<String>, pool: &MySqlPool) {
    todo!()
}

pub async fn save_meme_to_file(name: &str, bytes: &Vec<u8>, folder: &str) -> String {
    todo!()
}

pub async fn create_new_category_dirs(categories: &Vec<String>) {
    todo!()
}

pub async fn add_categories_to_hash(categories: &Vec<String>, hash: u64, pool: &MySqlPool) {
    todo!()
}

pub async fn hash_exists(hash: u64, pool: &MySqlPool) -> anyhow::Result<bool> {
    let hash = sqlx::query_as!(
        FileHash,
        "select hash, path from file_hashes where hash = ?",
        hash
    )
    .fetch_optional(pool)
    .await?;

    Ok(hash.is_some())
}

#[tracing::instrument(ret)]
pub fn get_meme_folders() -> Vec<DirEntry> {
    let Ok(folders) = fs::read_dir(MEMES_FOLDER) else {
        return vec![];
    };

    folders
        .filter_map(std::result::Result::ok)
        .filter(|f| f.file_type().is_ok_and(|f| f.is_dir()))
        .collect_vec()
}

#[tracing::instrument(ret)]
pub fn get_random_meme_folders() -> Vec<DirEntry> {
    let Ok(folders) = fs::read_dir(RANDOM_MEMES_FOLDER) else {
        return vec![];
    };

    folders
        .filter_map(std::result::Result::ok)
        .filter(|f| f.file_type().is_ok_and(|f| f.is_dir()))
        .collect_vec()
}
