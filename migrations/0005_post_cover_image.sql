-- Add optional cover image path to posts.
ALTER TABLE posts ADD COLUMN cover_image TEXT NOT NULL DEFAULT '';