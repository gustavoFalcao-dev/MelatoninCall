-- Add migration script here
ALTER TABLE channels 
ADD CONSTRAINT unique_channel_name_per_server UNIQUE (server_id, name);