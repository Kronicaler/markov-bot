-- Add migration script here
CREATE TABLE IF NOT EXISTS tag_blacklisted_users
(
    user_id          BIGINT UNSIGNED PRIMARY KEY NOT NULL
);