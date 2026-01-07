CREATE TABLE IF NOT EXISTS meme_categories (
    id SERIAL PRIMARY KEY,
    category TEXT UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS meme_files (
    id SERIAL PRIMARY KEY,
    folder TEXT NOT NULL,
    name TEXT NOT NULL,
    extension TEXT NOT NULL,
    hash BIGINT UNIQUE NOT NULL
);

CREATE UNIQUE INDEX mf_folder_name on meme_files (folder, name);

-- track the last file of a category posted to a discord server
CREATE TABLE IF NOT EXISTS meme_server_categories (
    server_id BIGINT NOT NULL,
    category_id INT NOT NULL,
    file_id INT NOT NULL,
    PRIMARY KEY(server_id, category_id),
    FOREIGN KEY(category_id) REFERENCES meme_categories(id),
    FOREIGN KEY(file_id) REFERENCES meme_files(id)
);

CREATE TABLE IF NOT EXISTS meme_file_categories (
    category_id INT NOT NULL,
    file_id INT NOT NULL,
    PRIMARY KEY(category_id, file_id),
    FOREIGN KEY(file_id) REFERENCES meme_files(id),
    FOREIGN KEY(category_id) REFERENCES meme_categories(id)
);