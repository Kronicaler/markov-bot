-- Add migration script here
CREATE TABLE IF NOT EXISTS tag_bans (
    user_id BIGINT NOT NULL,
    server_id BIGINT NOT NULL,
    PRIMARY KEY(user_id, server_id)
);