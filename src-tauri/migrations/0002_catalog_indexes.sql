CREATE INDEX IF NOT EXISTS idx_packages_total_downloads
  ON packages (total_downloads DESC);

CREATE INDEX IF NOT EXISTS idx_packages_full_name
  ON packages (full_name);

CREATE INDEX IF NOT EXISTS idx_packages_author
  ON packages (author);

CREATE INDEX IF NOT EXISTS idx_reference_overrides_package_version
  ON reference_overrides (package_id, version_id);
