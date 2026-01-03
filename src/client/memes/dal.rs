use std::{
    fs::{self, DirEntry},
    path::PathBuf,
};

use file_format::FileFormat;
use itertools::Itertools;
use sqlx::PgPool;

use crate::client::memes::MEMES_FOLDER;

#[derive(Debug)]
pub struct MemeServerCategory {
    pub server_id: i64,
    pub category_id: i32,
    pub file_id: i32,
}

pub struct MemeFile {
    pub id: i32,
    pub folder: String,
    pub name: String,
    pub hash: i64,
}

pub struct MemeFileCategory {
    pub category_id: i32,
    pub file_id: i32,
}

pub struct MemeCategory {
    pub id: i32,
    pub category: String,
}

#[tracing::instrument(err, skip(pool))]
pub async fn get_file_by_id(id: i32, pool: &PgPool) -> anyhow::Result<Option<MemeFile>> {
    Ok(sqlx::query_as!(
        MemeFile,
        r#"
            SELECT * FROM meme_files
            WHERE id = $1
            "#,
        id,
    )
    .fetch_optional(pool)
    .await?)
}

#[tracing::instrument(err, skip(pool))]
pub async fn get_category_by_name(
    category: &str,
    pool: &PgPool,
) -> anyhow::Result<Option<MemeCategory>> {
    Ok(sqlx::query_as!(
        MemeCategory,
        r#"
            SELECT * FROM meme_categories
            WHERE category = $1
            "#,
        category,
    )
    .fetch_optional(pool)
    .await?)
}

#[tracing::instrument(err, skip(pool))]
pub async fn get_server_category(
    server_id: i64,
    category_id: i32,
    pool: &PgPool,
) -> anyhow::Result<Option<MemeServerCategory>> {
    Ok(sqlx::query_as!(
        MemeServerCategory,
        r#"
            SELECT * FROM meme_server_categories
            WHERE category_id = $1 AND server_id = $2
            "#,
        category_id,
        server_id
    )
    .fetch_optional(pool)
    .await?)
}

#[tracing::instrument(err, skip(pool))]
pub async fn set_server_category(
    server_id: i64,
    category_id: i32,
    file_id: i32,
    pool: &PgPool,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
		INSERT INTO meme_server_categories ( server_id, category_id, file_id )
		VALUES ( $1, $2, $3 )
        ON CONFLICT(server_id, category_id)
        DO UPDATE SET
            file_id = EXCLUDED.file_id
		"#,
        server_id,
        category_id,
        file_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn save_meme_to_file(name: &str, bytes: &Vec<u8>, folder: &str) -> anyhow::Result<()> {
    let mut path = PathBuf::new();
    path.push(folder);
    path.push(name);

    let ext = FileFormat::from_bytes(bytes);
    path.set_extension(ext.extension());

    fs::write(&path, bytes)?;

    Ok(())
}

pub async fn create_new_category_dirs(categories: &Vec<String>) -> anyhow::Result<()> {
    for category in categories {
        let category_dir = format!("{MEMES_FOLDER}/{category}");
        fs::create_dir(&category_dir)?;
    }

    Ok(())
}

pub async fn hash_exists(hash: i64, pool: &PgPool) -> anyhow::Result<bool> {
    let file = sqlx::query_as!(MemeFile, "select * from meme_files where hash = $1", hash)
        .fetch_optional(pool)
        .await?;

    Ok(file.is_some())
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

pub async fn create_meme_file(
    folder: &str,
    name: &str,
    hash: i64,
    pool: &sqlx::Pool<sqlx::Postgres>,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
		INSERT INTO meme_files ( folder, name, hash )
		VALUES ( $1, $2, $3 )
		"#,
        folder,
        name,
        hash
    )
    .execute(pool)
    .await?;

    Ok(())
}
