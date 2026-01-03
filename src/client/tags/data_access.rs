use sqlx::PgPool;
use thiserror::Error;

use super::{
    Tag,
    model::{TagBlacklistedUser, TagChannel},
};

#[derive(Debug, Error)]
pub enum CreateTagError {
    #[error("There already exists a tag with the same listener")]
    TagWithSameListenerExists,
}

pub async fn create_tag(
    listener: String,
    response: String,
    creator_name: String,
    creator_id: i64,
    server_id: i64,
    pool: &PgPool,
) -> Result<Tag, CreateTagError> {
    let created_tag_id = sqlx::query!(
        r#"
		INSERT INTO tags ( listener, response, creator_name, creator_id, server_id )
		VALUES ( $1, $2, $3, $4, $5)
        RETURNING id
		"#,
        listener,
        response,
        creator_name,
        creator_id,
        server_id
    )
    .fetch_one(pool)
    .await
    .or(Err(CreateTagError::TagWithSameListenerExists))?
    .id;

    Ok(get_tag_by_id(created_tag_id, pool).await.unwrap())
}

pub async fn delete_tag(id: i32, pool: &PgPool) -> u64 {
    sqlx::query!(
        r#"
        DELETE FROM tags
        WHERE id= $1
        "#,
        id
    )
    .execute(pool)
    .await
    .unwrap()
    .rows_affected()
}

pub async fn get_tag_by_listener(listener: &str, server_id: i64, pool: &PgPool) -> Option<Tag> {
    sqlx::query_as!(
        Tag,
        r#"
        SELECT * FROM tags
        WHERE listener = $1 AND server_id = $2
        "#,
        listener,
        server_id
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}

pub async fn get_tag_by_id(id: i32, pool: &PgPool) -> Option<Tag> {
    sqlx::query_as!(
        Tag,
        r#"
        SELECT * FROM tags
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}

pub async fn get_tags_by_server_id(server_id: i64, pool: &PgPool) -> Vec<Tag> {
    sqlx::query_as!(
        Tag,
        r#"
        SELECT * FROM tags
        WHERE server_id = $1
        "#,
        server_id
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

pub async fn get_tag_blacklisted_user(user_id: i64, pool: &PgPool) -> Option<TagBlacklistedUser> {
    sqlx::query_as!(
        TagBlacklistedUser,
        r#"
        SELECT * FROM tag_blacklisted_users
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}

pub async fn delete_tag_blacklisted_user(user_id: i64, pool: &PgPool) -> u64 {
    sqlx::query!(
        r#"
        DELETE FROM tag_blacklisted_users
        WHERE user_id= $1
        "#,
        user_id
    )
    .execute(pool)
    .await
    .unwrap()
    .rows_affected()
}

pub async fn create_tag_blacklisted_user(user_id: i64, pool: &PgPool) -> TagBlacklistedUser {
    sqlx::query!(
        r#"
		INSERT INTO tag_blacklisted_users ( user_id )
		VALUES ( $1 )
		"#,
        user_id
    )
    .execute(pool)
    .await
    .unwrap();

    get_tag_blacklisted_user(user_id, pool).await.unwrap()
}

pub async fn get_tag_channel(server_id: i64, pool: &PgPool) -> Option<TagChannel> {
    sqlx::query_as!(
        TagChannel,
        r#"
        SELECT * FROM tag_channels
        WHERE server_id = $1
        "#,
        server_id,
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}

pub async fn update_tag_channel(server_id: i64, channel_id: i64, pool: &PgPool) -> u64 {
    sqlx::query!(
        r#"
        UPDATE tag_channels
        SET channel_id = $1
        WHERE server_id = $2
        "#,
        channel_id,
        server_id,
    )
    .execute(pool)
    .await
    .unwrap()
    .rows_affected()
}

pub async fn create_tag_channel(server_id: i64, channel_id: i64, pool: &PgPool) -> TagChannel {
    sqlx::query!(
        r#"
		INSERT INTO tag_channels ( server_id, channel_id )
		VALUES ( $1, $2 )
		"#,
        server_id,
        channel_id
    )
    .execute(pool)
    .await
    .unwrap();

    get_tag_channel(server_id, pool).await.unwrap()
}
