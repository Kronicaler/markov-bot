use sqlx::MySqlPool;
use thiserror::Error;

use super::{global_data::TagBlacklistedUser, Tag};

#[derive(Debug, Error)]
pub enum CreateTagError {
    #[error("There already exists a tag with the same listener")]
    TagWithSameListenerExists,
}

pub async fn create_tag(
    listener: String,
    response: String,
    creator_name: String,
    creator_id: u64,
    server_id: u64,
    pool: &MySqlPool,
) -> Result<Tag, CreateTagError> {
    let created_tag_id = sqlx::query!(
        r#"
		INSERT INTO tags ( listener, response, creator_name, creator_id, server_id )
		VALUES ( ?, ?, ?, ?, ?)
		"#,
        listener,
        response,
        creator_name,
        creator_id,
        server_id
    )
    .execute(pool)
    .await
    .or(Err(CreateTagError::TagWithSameListenerExists))?
    .last_insert_id();

    Ok(get_tag_by_id(created_tag_id, pool).await.unwrap())
}

pub async fn delete_tag(id: u64, pool: &MySqlPool) -> u64 {
    sqlx::query!(
        r#"
        DELETE FROM tags
        WHERE id= ?
        "#,
        id
    )
    .execute(pool)
    .await
    .unwrap()
    .rows_affected()
}

pub async fn get_tag_by_listener(listener: &str, server_id: u64, pool: &MySqlPool) -> Option<Tag> {
    sqlx::query_as!(
        Tag,
        r#"
        SELECT * FROM tags
        WHERE listener = ? AND server_id = ?
        "#,
        listener,
        server_id
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}

pub async fn get_tag_by_id(id: u64, pool: &MySqlPool) -> Option<Tag> {
    sqlx::query_as!(
        Tag,
        r#"
        SELECT * FROM tags
        WHERE id = ?
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}

pub async fn get_tags_by_server_id(server_id: u64, pool: &MySqlPool) -> Vec<Tag> {
    sqlx::query_as!(
        Tag,
        r#"
        SELECT * FROM tags
        WHERE server_id = ?
        "#,
        server_id
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

pub async fn get_tag_blacklisted_user(
    user_id: u64,
    pool: &MySqlPool,
) -> Option<TagBlacklistedUser> {
    sqlx::query_as!(
        TagBlacklistedUser,
        r#"
        SELECT * FROM tag_blacklisted_users
        WHERE user_id = ?
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}

pub async fn delete_tag_blacklisted_user(user_id: u64, pool: &MySqlPool) -> u64 {
    sqlx::query!(
        r#"
        DELETE FROM tag_blacklisted_users
        WHERE user_id= ?
        "#,
        user_id
    )
    .execute(pool)
    .await
    .unwrap()
    .rows_affected()
}

pub async fn create_tag_blacklisted_user(user_id: u64, pool: &MySqlPool) -> TagBlacklistedUser {
    sqlx::query!(
        r#"
		INSERT INTO tag_blacklisted_users ( user_id )
		VALUES ( ? )
		"#,
        user_id
    )
    .execute(pool)
    .await
    .unwrap();

    get_tag_blacklisted_user(user_id, pool)
        .await
        .unwrap()
}
