-- Your SQL goes here
ALTER TABLE events ADD COLUMN chain VARCHAR NOT NULL DEFAULT 'default';
ALTER TABLE events ALTER COLUMN chain DROP DEFAULT;
