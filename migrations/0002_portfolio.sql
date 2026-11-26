-- Portfolio content: profile, socials, projects, toolkit, principles, milestones.
-- Everything here is editable from /admin/content, nothing is compiled in.
CREATE TABLE IF NOT EXISTS profile (
  id            INTEGER PRIMARY KEY CHECK (id = 1),
  name          TEXT NOT NULL,
  job_title     TEXT NOT NULL,
  description   TEXT NOT NULL,
  location      TEXT NOT NULL,
  email         TEXT NOT NULL,
  availability  TEXT NOT NULL,
  hero_lead     TEXT NOT NULL,
  hero_focus    TEXT NOT NULL,
  hero_focus_detail TEXT NOT NULL,
  about_intro   TEXT NOT NULL,
  updated_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS social_links (
  id       TEXT PRIMARY KEY,
  label    TEXT NOT NULL,
  handle   TEXT NOT NULL DEFAULT '',
  href     TEXT NOT NULL,
  icon     TEXT NOT NULL DEFAULT 'link',
  position INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS project_categories (
  id       TEXT PRIMARY KEY,
  label    TEXT NOT NULL,
  position INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS projects (
  id             TEXT PRIMARY KEY,
  slug           TEXT NOT NULL UNIQUE,
  name           TEXT NOT NULL,
  category       TEXT NOT NULL DEFAULT 'mobile',
  category_label TEXT NOT NULL DEFAULT '',
  kicker         TEXT NOT NULL DEFAULT '',
  tagline        TEXT NOT NULL DEFAULT '',
  summary        TEXT NOT NULL DEFAULT '',
  year           TEXT NOT NULL DEFAULT '',
  role           TEXT NOT NULL DEFAULT '',
  status         TEXT NOT NULL DEFAULT '',
  accent         TEXT NOT NULL DEFAULT 'lime',
  stack          TEXT NOT NULL DEFAULT '[]',
  features       TEXT NOT NULL DEFAULT '[]',
  story          TEXT NOT NULL DEFAULT '[]',
  outcome        TEXT NOT NULL DEFAULT '[]',
  link_label     TEXT NOT NULL DEFAULT '',
  link_href      TEXT NOT NULL DEFAULT '',
  visible        INTEGER NOT NULL DEFAULT 1,
  position       INTEGER NOT NULL DEFAULT 0,
  created_at     TEXT NOT NULL,
  updated_at     TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS projects_visible_order
  ON projects (visible, position);

CREATE TABLE IF NOT EXISTS tool_groups (
  id       TEXT PRIMARY KEY,
  number   TEXT NOT NULL DEFAULT '',
  title    TEXT NOT NULL,
  subtitle TEXT NOT NULL DEFAULT '',
  position INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS tools (
  id           TEXT PRIMARY KEY,
  name         TEXT NOT NULL,
  group_id     TEXT NOT NULL REFERENCES tool_groups (id) ON DELETE CASCADE,
  description  TEXT NOT NULL DEFAULT '',
  project_slug TEXT NOT NULL DEFAULT '',
  position     INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS tools_group_order
  ON tools (group_id, position);

CREATE TABLE IF NOT EXISTS principles (
  id       TEXT PRIMARY KEY,
  title    TEXT NOT NULL,
  body     TEXT NOT NULL DEFAULT '',
  position INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS milestones (
  id       TEXT PRIMARY KEY,
  marker   TEXT NOT NULL DEFAULT '',
  title    TEXT NOT NULL,
  detail   TEXT NOT NULL DEFAULT '',
  position INTEGER NOT NULL DEFAULT 0
);
