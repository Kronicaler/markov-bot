-- Add migration script here
CREATE TABLE IF NOT EXISTS markov_blacklisted_servers
(
    server_id          BIGINT UNSIGNED PRIMARY KEY NOT NULL
);