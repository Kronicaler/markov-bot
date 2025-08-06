use sqlx::MySqlPool;

#[derive(Debug)]
pub struct ServerFolderIndex {
    pub server_id: u64,
    pub folder_name: String,
    pub file_index: u32,
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
