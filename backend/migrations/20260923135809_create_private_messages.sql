-- Add migration script here
CREATE TABLE private_messages (
    id UUID PRIMARY KEY,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    receiver_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    image_data BYTEA,
    image_mime_type VARCHAR(50),
    image_expires_at TIMESTAMPTZ,
    is_image_expired BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE messages ADD COLUMN image_data BYTEA;
ALTER TABLE messages ADD COLUMN image_mime_type VARCHAR(50);
ALTER TABLE messages ADD COLUMN image_expires_at TIMESTAMPTZ;
ALTER TABLE messages ADD COLUMN is_image_expired BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE messages ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();