-- Add migration script here
CREATE TABLE IF NOT EXISTS server_folder_indexes
(
    server_id          BIGINT UNSIGNED NOT NULL,
	folder_name		   TEXT NOT NULL,
    file_index         INT UNSIGNED NOT NULL 
);

CREATE UNIQUE INDEX sfi on server_folder_indexes(server_id, folder_name);