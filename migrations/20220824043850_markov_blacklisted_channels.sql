-- Add migration script here
CREATE TABLE IF NOT EXISTS markov_blacklisted_channels
(
    channel_id          BIGINT PRIMARY KEY NOT NULL
);