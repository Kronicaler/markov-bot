-- track the last file of a category posted to a server
CREATE TABLE IF NOT EXISTS meme_server_category_indexes (
    server_id BIGINT UNSIGNED NOT NULL,
    category TEXT NOT NULL,
    tag_index INT UNSIGNED NOT NULL
);

CREATE UNIQUE INDEX sfi on meme_server_folder_indexes(server_id, category);

CREATE TABLE IF NOT EXISTS meme_file_hashes (
    hash BIGINT UNSIGNED NOT NULL PRIMARY KEY,
    path TEXT UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS meme_file_categories (
    category_id INT NOT NULL,
    file_hash BIGINT UNSIGNED NOT NULL,
    FOREIGN KEY(file_hash) REFERENCES file_hashes(hash)
    FOREIGN KEY(category_id) REFERENCES categories(id)
);

CREATE TABLE IF NOT EXISTS meme_categories (
    id INT PRIMARY KEY NOT NULL,
    category TEXT PRIMARY KEY NOT NULL
);