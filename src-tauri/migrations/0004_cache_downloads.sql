CREATE TABLE IF NOT EXISTS cached_archives (
  cache_key TEXT PRIMARY KEY,
  source_kind TEXT NOT NULL,
  package_id TEXT NULL REFERENCES packages(id) ON DELETE RESTRICT,
  version_id TEXT NULL REFERENCES package_versions(id) ON DELETE RESTRICT,
  sha256 TEXT NOT NULL,
  archive_name TEXT NOT NULL,
  relative_path TEXT NOT NULL,
  file_size INTEGER NOT NULL,
  source_url TEXT NULL,
  first_cached_at TEXT NOT NULL,
  last_used_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_cached_archives_version_id
ON cached_archives(version_id)
WHERE version_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_cached_archives_sha256
ON cached_archives(sha256);

CREATE INDEX IF NOT EXISTS idx_cached_archives_last_used_at
ON cached_archives(last_used_at);

CREATE TABLE IF NOT EXISTS install_tasks (
  id TEXT PRIMARY KEY,
  profile_id TEXT NULL,
  kind TEXT NOT NULL,
  status TEXT NOT NULL,
  title TEXT NOT NULL,
  detail TEXT NOT NULL,
  progress_step TEXT NULL,
  progress_current INTEGER NOT NULL,
  progress_total INTEGER NOT NULL,
  error_message TEXT NULL,
  created_at TEXT NOT NULL,
  started_at TEXT NULL,
  finished_at TEXT NULL
);

CREATE INDEX IF NOT EXISTS idx_install_tasks_kind_status
ON install_tasks(kind, status);

CREATE TABLE IF NOT EXISTS download_jobs (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES install_tasks(id) ON DELETE CASCADE,
  package_name TEXT NOT NULL,
  version_label TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  cache_hit INTEGER NOT NULL,
  bytes_downloaded INTEGER NOT NULL,
  total_bytes INTEGER NULL,
  speed_bps INTEGER NULL,
  progress_label TEXT NOT NULL,
  error_message TEXT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_download_jobs_task_id
ON download_jobs(task_id);
