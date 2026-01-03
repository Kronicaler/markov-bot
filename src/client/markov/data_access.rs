use sqlx::{PgPool, postgres::PgQueryResult, query, query_as};

use crate::client::markov::model::MarkovBlacklistedServer;

use super::model::{MarkovBlacklistedChannel, MarkovBlacklistedUser};

pub async fn get_markov_blacklisted_server(
    server_id: i64,
    pool: &PgPool,
) -> Option<MarkovBlacklistedServer> {
    query_as!(
        MarkovBlacklistedServer,
        "
		SELECT * FROM markov_blacklisted_servers where server_id = $1
		",
        server_id
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}

pub async fn create_markov_blacklisted_server(
    server_id: i64,
    pool: &PgPool,
) -> anyhow::Result<PgQueryResult> {
    Ok(query!(
        r#"
		INSERT INTO markov_blacklisted_servers (server_id)
		VALUES ($1)
		"#,
        server_id
    )
    .execute(pool)
    .await?)
}

pub async fn delete_markov_blacklisted_server(
    server_id: i64,
    pool: &PgPool,
) -> anyhow::Result<PgQueryResult> {
    Ok(query!(
        r#"
		DELETE FROM markov_blacklisted_servers 
		WHERE server_id = $1
		"#,
        server_id
    )
    .execute(pool)
    .await?)
}

pub async fn get_markov_blacklisted_user(
    user_id: i64,
    pool: &PgPool,
) -> Option<MarkovBlacklistedUser> {
    query_as!(
        MarkovBlacklistedUser,
        "
		SELECT * FROM markov_blacklisted_users where user_id = $1
		",
        user_id
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}

pub async fn create_markov_blacklisted_user(
    user_id: i64,
    pool: &PgPool,
) -> anyhow::Result<PgQueryResult> {
    Ok(query!(
        r#"
		INSERT INTO markov_blacklisted_users (user_id)
		VALUES ($1)
		"#,
        user_id
    )
    .execute(pool)
    .await?)
}

pub async fn delete_markov_blacklisted_user(
    user_id: i64,
    pool: &PgPool,
) -> anyhow::Result<PgQueryResult> {
    Ok(query!(
        r#"
		DELETE FROM markov_blacklisted_users 
		WHERE user_id = $1
		"#,
        user_id
    )
    .execute(pool)
    .await?)
}

pub async fn get_markov_blacklisted_channel(
    channel_id: i64,
    pool: &PgPool,
) -> Option<MarkovBlacklistedChannel> {
    query_as!(
        MarkovBlacklistedChannel,
        "
		SELECT * FROM markov_blacklisted_channels where channel_id = $1
		",
        channel_id
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}

pub async fn create_markov_blacklisted_channel(
    channel_id: i64,
    pool: &PgPool,
) -> anyhow::Result<PgQueryResult> {
    Ok(query!(
        r#"
		INSERT INTO markov_blacklisted_channels (channel_id)
		VALUES ($1)
		"#,
        channel_id
    )
    .execute(pool)
    .await?)
}

pub async fn delete_markov_blacklisted_channel(
    channel_id: i64,
    pool: &PgPool,
) -> anyhow::Result<PgQueryResult> {
    Ok(query!(
        r#"
		DELETE FROM markov_blacklisted_channels
		WHERE channel_id = $1
		"#,
        channel_id
    )
    .execute(pool)
    .await?)
}
