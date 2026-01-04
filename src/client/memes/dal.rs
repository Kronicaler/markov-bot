use std::{
    fs::{self, DirEntry},
    path::PathBuf,
};

use file_format::FileFormat;
use itertools::Itertools;
use sqlx::PgConnection;

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

#[tracing::instrument(err, skip(conn))]
pub async fn get_file_by_id(id: i32, conn: &mut PgConnection) -> anyhow::Result<Option<MemeFile>> {
    Ok(sqlx::query_as!(
        MemeFile,
        r#"
            SELECT * FROM meme_files
            WHERE id = $1
            "#,
        id,
    )
    .fetch_optional(conn)
    .await?)
}

#[tracing::instrument(err, skip(conn))]
pub async fn get_categories_by_name(
    categories: &[String],
    conn: &mut PgConnection,
) -> anyhow::Result<Vec<MemeCategory>> {
    Ok(sqlx::query_as!(
        MemeCategory,
        r#"
            SELECT * FROM meme_categories
            WHERE category = any($1)
            "#,
        &categories,
    )
    .fetch_all(conn)
    .await?)
}

#[tracing::instrument(err, skip(conn))]
pub async fn get_server_category(
    server_id: i64,
    category_id: i32,
    conn: &mut PgConnection,
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
    .fetch_optional(conn)
    .await?)
}

#[tracing::instrument(err, skip(conn))]
pub async fn set_server_category(
    server_id: i64,
    category_id: i32,
    file_id: i32,
    conn: &mut PgConnection,
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
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn save_meme_to_file(name: &str, bytes: &Vec<u8>, folder: &str) -> anyhow::Result<()> {
    let mut path = PathBuf::new();
    path.push(MEMES_FOLDER);
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

pub async fn get_file_by_hash(
    hash: i64,
    conn: &mut PgConnection,
) -> anyhow::Result<Option<MemeFile>> {
    let file = sqlx::query_as!(MemeFile, "select * from meme_files where hash = $1", hash)
        .fetch_optional(conn)
        .await?;

    Ok(file)
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
    conn: &mut PgConnection,
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
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn create_new_categories(
    categories: &[String],
    conn: &mut PgConnection,
) -> anyhow::Result<()> {
    for category in categories {
        sqlx::query!(
            r#"
        INSERT INTO meme_categories ( category )
        VALUES ( $1 )
        ON CONFLICT(category) DO NOTHING
        "#,
            category
        )
        .execute(&mut *conn)
        .await?;
    }

    Ok(())
}

pub async fn create_meme_file_categories(
    categories: &[String],
    meme_file_id: i32,
    conn: &mut PgConnection,
) -> anyhow::Result<()> {
    let categories = get_categories_by_name(categories, conn).await?;

    for category in categories {
        sqlx::query!(
            r#"
            INSERT INTO meme_file_categories ( category_id, file_id )
            VALUES ( $1, $2 )
            ON CONFLICT(category_id, file_id)
            DO NOTHING
            "#,
            category.id,
            meme_file_id
        )
        .execute(&mut *conn)
        .await?;
    }

    Ok(())
}

pub async fn get_meme_file_count_by_folder(
    folder: &str,
    conn: &mut PgConnection,
) -> anyhow::Result<i64> {
    Ok(sqlx::query!(
        r#"
            select count(*) from meme_files where folder = $1
            "#,
        folder
    )
    .fetch_one(&mut *conn)
    .await?
    .count
    .unwrap_or(0))
}
