CREATE TABLE contracts (
    id serial PRIMARY KEY,
    uuid VARCHAR NOT NULL,
    state VARCHAR NOT NULL,
    content TEXT NOT NULL,
    key VARCHAR NOT NULL,
    UNIQUE (key, uuid)
);
