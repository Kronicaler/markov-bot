-- Add migration script here
ALTER TABLE tags
ADD server_id BIGINT UNSIGNED NOT NULL;

ALTER TABLE tags
DROP INDEX listener;

ALTER TABLE tags
ADD UNIQUE tag_by_server( listener, server_id );