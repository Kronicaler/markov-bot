-- Add migration script here
-- Add migration script here
CREATE TABLE IF NOT EXISTS tag_channels
(
    server_id          BIGINT PRIMARY KEY NOT NULL,
	channel_id		   BIGINT NOT NULL
);