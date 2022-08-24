-- Add migration script here
CREATE TABLE IF NOT EXISTS markov_blacklisted_channels
(
    channel_id          BIGINT UNSIGNED PRIMARY KEY NOT NULL
);