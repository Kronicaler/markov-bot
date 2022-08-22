CREATE TABLE IF NOT EXISTS tags
(
    id          	BIGINT UNSIGNED PRIMARY KEY NOT NULL AUTO_INCREMENT,
    listener		TEXT UNIQUE		 NOT NULL,
    response		TEXT			 NOT NULL,
    creator_name	TEXT			 NOT NULL,
	creator_id		BIGINT UNSIGNED NOT NULL
);

-- CREATE TABLE IF NOT EXISTS tag_users
-- (
--     id          BIGINT UNSIGNED PRIMARY KEY NOT NULL AUTO_INCREMENT,
--     description TEXT    NOT NULL,
--     done        BOOLEAN NOT NULL DEFAULT FALSE
-- );
