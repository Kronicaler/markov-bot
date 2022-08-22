use sqlx::MySqlPool;
use thiserror::Error;

use super::Tag;

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
    pool: &MySqlPool,
) -> Result<Tag, CreateTagError> {
    let created_tag_id = sqlx::query!(
        r#"
		INSERT INTO tags ( listener, response, creator_name, creator_id )
		VALUES ( ?, ?, ?, ?)
		"#,
        listener,
        response,
        creator_name,
        creator_id
    )
    .execute(pool)
    .await
    .or(Err(CreateTagError::TagWithSameListenerExists))?
    .last_insert_id();

    Ok(get_tag_by_id(created_tag_id, pool).await.unwrap())
}

pub async fn update_tag(tag: Tag, pool: &MySqlPool) -> Tag {
    let created_tag_id = sqlx::query!(
        r#"
		UPDATE tags
		SET listener = ?, response = ?, creator_name = ?, creator_id = ?
        WHERE id = ?
		"#,
        tag.listener,
        tag.response,
        tag.creator_name,
        tag.creator_id,
        tag.id
    )
    .execute(pool)
    .await
    .unwrap()
    .last_insert_id();

    get_tag_by_id(created_tag_id, pool).await.unwrap()
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

pub async fn get_tag_by_listener(listener: &str, pool: &MySqlPool) -> Option<Tag> {
    sqlx::query_as!(
        Tag,
        r#"
        SELECT * FROM tags
        WHERE listener = ?
        "#,
        listener
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

pub async fn get_all_tags(pool: &MySqlPool) -> Vec<Tag> {
    sqlx::query_as!(
        Tag,
        r#"
        SELECT * FROM tags
        "#,
    )
    .fetch_all(pool)
    .await
    .unwrap()
}