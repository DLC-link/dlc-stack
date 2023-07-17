CREATE TABLE events (
    id serial PRIMARY KEY,
    event_id VARCHAR NOT NULL,
    content TEXT NOT NULL,
    key VARCHAR NOT NULL,
    UNIQUE (key, event_id)
);
