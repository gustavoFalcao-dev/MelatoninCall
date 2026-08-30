-- Add migration script here
TRUNCATE TABLE users CASCADE;
ALTER TABLE users ADD COLUMN email TEXT NOT NULL UNIQUE;