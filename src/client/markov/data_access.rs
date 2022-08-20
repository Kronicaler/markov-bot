use sqlx::{query, query_as, MySqlPool, mysql::MySqlQueryResult};

use crate::client::markov::global_data::MarkovBlacklistedServer;

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

pub async fn create_markov_blacklisted_server(server_id: u64, pool: &MySqlPool) -> anyhow::Result<MySqlQueryResult> {
    Ok(query!(
        r#"
		INSERT INTO markov_blacklisted_servers (server_id)
		VALUES (?)
		"#,
        server_id
    ).execute(pool).await?)
}

pub async fn delete_markov_blacklisted_server(server_id: u64, pool: &MySqlPool) -> anyhow::Result<MySqlQueryResult> {
    Ok(query!(
        r#"
		DELETE FROM markov_blacklisted_servers 
		WHERE server_id = ?
		"#,
        server_id
    ).execute(pool).await?)
}