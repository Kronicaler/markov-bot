use sqlx::{mysql::MySqlQueryResult, query, query_as, MySqlPool};

use crate::client::markov::model::MarkovBlacklistedServer;

use super::model::{MarkovBlacklistedChannel, MarkovBlacklistedUser};

pub async fn get_markov_blacklisted_server(
    server_id: u64,
    pool: &MySqlPool,
) -> Option<MarkovBlacklistedServer> {
    query_as!(
        MarkovBlacklistedServer,
        "
		SELECT * FROM markov_blacklisted_servers where server_id = ?
		",
        server_id
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}

pub async fn create_markov_blacklisted_server(
    server_id: u64,
    pool: &MySqlPool,
) -> anyhow::Result<MySqlQueryResult> {
    Ok(query!(
        r#"
		INSERT INTO markov_blacklisted_servers (server_id)
		VALUES (?)
		"#,
        server_id
    )
    .execute(pool)
    .await?)
}

pub async fn delete_markov_blacklisted_server(
    server_id: u64,
    pool: &MySqlPool,
) -> anyhow::Result<MySqlQueryResult> {
    Ok(query!(
        r#"
		DELETE FROM markov_blacklisted_servers 
		WHERE server_id = ?
		"#,
        server_id
    )
    .execute(pool)
    .await?)
}

pub async fn get_markov_blacklisted_user(
    user_id: u64,
    pool: &MySqlPool,
) -> Option<MarkovBlacklistedUser> {
    query_as!(
        MarkovBlacklistedUser,
        "
		SELECT * FROM markov_blacklisted_users where user_id = ?
		",
        user_id
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}

pub async fn create_markov_blacklisted_user(
    user_id: u64,
    pool: &MySqlPool,
) -> anyhow::Result<MySqlQueryResult> {
    Ok(query!(
        r#"
		INSERT INTO markov_blacklisted_users (user_id)
		VALUES (?)
		"#,
        user_id
    )
    .execute(pool)
    .await?)
}

pub async fn delete_markov_blacklisted_user(
    user_id: u64,
    pool: &MySqlPool,
) -> anyhow::Result<MySqlQueryResult> {
    Ok(query!(
        r#"
		DELETE FROM markov_blacklisted_users 
		WHERE user_id = ?
		"#,
        user_id
    )
    .execute(pool)
    .await?)
}

pub async fn get_markov_blacklisted_channel(
    channel_id: u64,
    pool: &MySqlPool,
) -> Option<MarkovBlacklistedChannel> {
    query_as!(
        MarkovBlacklistedChannel,
        "
		SELECT * FROM markov_blacklisted_channels where channel_id = ?
		",
        channel_id
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}

pub async fn create_markov_blacklisted_channel(
    channel_id: u64,
    pool: &MySqlPool,
) -> anyhow::Result<MySqlQueryResult> {
    Ok(query!(
        r#"
		INSERT INTO markov_blacklisted_channels (channel_id)
		VALUES (?)
		"#,
        channel_id
    )
    .execute(pool)
    .await?)
}

pub async fn delete_markov_blacklisted_channel(
    channel_id: u64,
    pool: &MySqlPool,
) -> anyhow::Result<MySqlQueryResult> {
    Ok(query!(
        r#"
		DELETE FROM markov_blacklisted_channels
		WHERE channel_id = ?
		"#,
        channel_id
    )
    .execute(pool)
    .await?)
}
