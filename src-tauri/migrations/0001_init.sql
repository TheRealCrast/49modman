PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS packages (
  id TEXT PRIMARY KEY,
  full_name TEXT NOT NULL UNIQUE,
  author TEXT NOT NULL,
  summary TEXT NOT NULL,
  categories_json TEXT NOT NULL,
  total_downloads INTEGER NOT NULL,
  rating REAL NOT NULL,
  website_url TEXT NOT NULL,
  synced_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS package_versions (
  id TEXT PRIMARY KEY,
  package_id TEXT NOT NULL REFERENCES packages(id) ON DELETE CASCADE,
  version_number TEXT NOT NULL,
  published_at TEXT NOT NULL,
  downloads INTEGER NOT NULL,
  base_zone TEXT NOT NULL,
  bundled_reference_state TEXT NULL,
  bundled_reference_note TEXT NULL,
  download_url TEXT NOT NULL,
  file_size INTEGER NULL,
  icon_url TEXT NULL,
  dependencies_json TEXT NOT NULL,
  sha256 TEXT NULL
);

CREATE INDEX IF NOT EXISTS idx_package_versions_package_published
  ON package_versions (package_id, published_at DESC);

CREATE INDEX IF NOT EXISTS idx_package_versions_package_version
  ON package_versions (package_id, version_number);

CREATE INDEX IF NOT EXISTS idx_package_versions_base_zone
  ON package_versions (base_zone);

CREATE TABLE IF NOT EXISTS reference_overrides (
  package_id TEXT NOT NULL REFERENCES packages(id) ON DELETE CASCADE,
  version_id TEXT NOT NULL REFERENCES package_versions(id) ON DELETE CASCADE,
  reference_state TEXT NOT NULL,
  note TEXT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (package_id, version_id)
);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sync_state (
  name TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
