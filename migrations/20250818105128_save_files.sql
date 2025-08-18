-- Add migration script here

CREATE TABLE IF NOT EXISTS file_hashes
(
    hash            BIGINT UNSIGNED NOT NULL PRIMARY KEY,
	path            TEXT UNIQUE NOT NULL,
);

CREATE TABLE IF NOT EXISTS file_categories
(
	category        TEXT NOT NULL,
    file_hash       BIGINT NOT NULL FOREIGN KEY REFERENCES file_hashes(hash),
);

CREATE INDEX file_categories_category on file_categories(category);
