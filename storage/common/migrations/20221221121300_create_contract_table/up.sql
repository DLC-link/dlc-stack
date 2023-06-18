CREATE TABLE contracts (
    id serial PRIMARY KEY,
    uuid VARCHAR NOT NULL UNIQUE,
    state VARCHAR NOT NULL,
    content TEXT NOT NULL,
    key VARCHAR NOT NULL
);
