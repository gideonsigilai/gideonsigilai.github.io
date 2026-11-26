-- Add optional cover image path to projects.
ALTER TABLE projects ADD COLUMN image TEXT NOT NULL DEFAULT '';
