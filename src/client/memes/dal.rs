use std::{
    fs::{self},
    path::PathBuf,
};

use sqlx::PgConnection;
use tracing::info;

use crate::client::memes::MEMES_FOLDER;

#[derive(Debug)]
pub struct MemeServerCategory {
    pub server_id: i64,
    pub category_id: i32,
    pub file_id: i32,
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

#[allow(unused)]
pub struct MemeFile {
    pub id: i32,
    pub folder: String,
    pub name: String,
    pub extension: String,
    pub hash: i64,
}

pub async fn create_meme_file(
    folder: &str,
    name: &str,
    extension: &str,
    hash: i64,
    conn: &mut PgConnection,
) -> anyhow::Result<i32> {
    Ok(sqlx::query!(
        r#"
            INSERT INTO meme_files ( folder, name, extension, hash )
            VALUES ( $1, $2, $3, $4 )
            RETURNING id
            "#,
        folder,
        name,
        extension,
        hash
    )
    .fetch_one(conn)
    .await?
    .id)
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

pub async fn get_file_by_hash(
    hash: i64,
    conn: &mut PgConnection,
) -> anyhow::Result<Option<MemeFile>> {
    let file = sqlx::query_as!(MemeFile, "select * from meme_files where hash = $1", hash)
        .fetch_optional(conn)
        .await?;

    Ok(file)
}

#[tracing::instrument(ret, err, skip(conn))]
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

#[derive(Debug, Clone)]
pub struct MemeFileCategory {
    pub category_id: i32,
    pub file_id: i32,
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

#[tracing::instrument(ret, err, skip(conn))]
pub async fn get_random_meme_file_category_by_category(
    category: &str,
    conn: &mut PgConnection,
) -> anyhow::Result<Option<MemeFileCategory>> {
    let category = get_categories_by_name(&[category.to_string()], conn)
        .await?
        .pop();

    let Some(category) = category else {
        info!("category doesn't exist");
        return Ok(None);
    };

    let meme_file_category = sqlx::query_as!(
        MemeFileCategory,
        r#"
            SELECT * FROM meme_file_categories
            WHERE category_id = $1
            ORDER BY RANDOM()
            LIMIT 1;
            "#,
        category.id,
    )
    .fetch_optional(&mut *conn)
    .await?;

    Ok(meme_file_category)
}

pub async fn get_oldest_meme_file_category(
    category_id: i32,
    conn: &mut PgConnection,
) -> anyhow::Result<Option<MemeFileCategory>> {
    let meme_file_category = sqlx::query_as!(
        MemeFileCategory,
        r#"
            SELECT * FROM meme_file_categories
            WHERE category_id = $1
            ORDER BY file_id
            LIMIT 1;
            "#,
        category_id,
    )
    .fetch_optional(&mut *conn)
    .await?;

    Ok(meme_file_category)
}

pub async fn get_next_meme_file_category(
    category_id: i32,
    file_id: i32,
    conn: &mut PgConnection,
) -> anyhow::Result<Option<MemeFileCategory>> {
    let meme_file_category = sqlx::query_as!(
        MemeFileCategory,
        r#"
            SELECT * FROM meme_file_categories
            WHERE category_id = $1 AND file_id > $2
            ORDER BY file_id
            LIMIT 1;
            "#,
        category_id,
        file_id
    )
    .fetch_optional(&mut *conn)
    .await?;

    Ok(meme_file_category)
}

#[allow(unused)]
pub struct MemeCategory {
    pub id: i32,
    pub category: String,
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

pub fn read_file(meme_file: &MemeFile) -> Result<Vec<u8>, anyhow::Error> {
    let mut path = PathBuf::new();
    path.push(MEMES_FOLDER);
    path.push(&meme_file.folder);
    path.push(&meme_file.name);
    path.set_extension(&meme_file.extension);

    Ok(fs::read(path)?)
}

#[tracing::instrument(err, skip(bytes))]
pub async fn save_meme_to_file(
    name: &str,
    extension: &str,
    bytes: &[u8],
    folder: &str,
) -> anyhow::Result<()> {
    let mut path = PathBuf::new();
    path.push(MEMES_FOLDER);
    path.push(folder);
    path.push(name);
    path.set_extension(extension);

    fs::write(&path, bytes)?;

    Ok(())
}

#[tracing::instrument(err)]
pub async fn create_new_category_dirs(categories: &Vec<String>) -> anyhow::Result<()> {
    for category in categories {
        let category_dir = format!("{MEMES_FOLDER}/{category}");
        if !fs::exists(&category_dir)? {
            fs::create_dir(&category_dir)?;
        }
    }

    Ok(())
}

pub struct CategoryCount {
    pub count: i64,
    pub category: String,
}

/// Get the number of files in each category ordered by descending count
pub async fn get_category_file_count(
    conn: &mut PgConnection,
) -> anyhow::Result<Vec<CategoryCount>> {
    let category_counts = sqlx::query_as!(
        CategoryCount,
        r#"
            SELECT coalesce(count(category), 0) as "count!", category
            FROM "meme_file_categories"
            INNER JOIN "meme_categories" ON "meme_categories"."id"= "category_id"
            group by "category"
            order by "count!" DESC 
            "#,
    )
    .fetch_all(&mut *conn)
    .await?;

    Ok(category_counts)
}
