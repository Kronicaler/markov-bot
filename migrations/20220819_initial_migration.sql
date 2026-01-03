CREATE TABLE IF NOT EXISTS tags (
    id SERIAL NOT NULL,
    listener TEXT NOT NULL,
    response TEXT NOT NULL,
    creator_name TEXT NOT NULL,
    creator_id BIGINT NOT NULL,
    server_id BIGINT NOT NULL,
    PRIMARY KEY ( listener, server_id )
);