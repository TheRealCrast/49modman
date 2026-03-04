# Cache System Milestone 1

## Summary

Implement the first real cache/download milestone for `49modman`.

This milestone makes the current Browse `Install` actions real, but only up to the cache boundary:

- when a user clicks `Install` or `Install version`, the app checks the global cache first
- if the exact package version is already cached, it reuses that cached archive
- if it is not cached, it downloads the archive from Thunderstore into the cache
- the action appears in the Downloads tab while it is active
- the cache can be opened from Settings in the system file explorer
- the cache can be cleared from Settings `Danger zone`

Locked decisions:

- install scope for this milestone: `Cache only`
- Downloads behavior: `Active only`
- Browse button label stays `Install`
- modpack/profile install state is still out of scope
- modpack import is still out of scope

## Goals

### Primary goals

- make `Install` and `Install version` perform real cache-aware work
- add a persistent global cache for exact Thunderstore versions
- make Downloads show active cache/download jobs
- add Settings access to open the cache folder
- add Settings `Danger zone` support to clear the cache

### Success criteria

This milestone is complete when:

1. Clicking `Install` or `Install version` creates a real backend task.
2. If the exact version is already cached and the file exists, the app does not redownload it.
3. If the version is not cached, the app downloads it from Thunderstore into the cache.
4. Downloads shows active queued/downloading/verifying/cache-hit work.
5. Completed successful jobs disappear from Downloads.
6. Settings can open the cache folder in the OS file explorer.
7. Settings can clear the cache after confirmation.
8. `Reset all data` also clears cache files now, not just SQLite state.
9. No profile/modpack install state is mutated yet.
10. Existing red/broken warning gates still apply before the cache/install action begins.

## Scope

### In scope

- global archive cache under app data
- cache lookup by exact Thunderstore `version_id`
- real Thunderstore archive download into cache
- active download/task persistence in SQLite
- Downloads tab wired to real backend jobs
- open cache folder from Settings
- clear cache from Settings `Danger zone`
- extend `reset_all_data` to wipe cache files too
- backend task dedupe for repeated requests of the same version while already downloading
- cache-hit fast path when archive is already present

### Out of scope

- profile/modpack mutation
- extracted mod files
- runtime assembly
- modpack import/export
- local ZIP import
- launch/activation
- Overview installed-mod state changes
- cache eviction policy beyond explicit `Clear cache`

## Current baseline

The current branch already has:

- real Tauri desktop runtime
- real backend-backed Browse/catalog
- real backend-backed profiles only
- placeholder `Install` actions in Browse
- mock Downloads screen state
- Settings `Warn options` and `Danger zone`
- `Reset all data` currently aimed at SQLite-backed user state

The current local environment also has a compatibility constraint:

- the user’s SQLite DB still contains leftover tables from the reverted install/cache experiment:
  - `cached_archives`
  - `install_tasks`
  - `download_jobs`
  - `profile_mods`
  - `profile_mod_dependencies`
  - `local_mods`

The new cache milestone must be compatible with that reality and not assume a perfectly clean DB.

## Product behavior

## Install behavior in this milestone

The Browse actions keep their current labels:

- package-level `Install`
- version-level `Install version`

But the actual behavior for this milestone is:

- cache the selected archive if needed
- do not install into a profile yet
- do not change Overview installed mods yet

### Package-level `Install`

Package-level `Install` still picks the target version using the existing priority order:

1. `verified`
2. `green`
3. `yellow`
4. `orange`
5. `red`
6. `broken`

### Warning behavior

The existing warning flow remains unchanged:

- `broken` versions still require the broken warning modal if enabled
- `red` versions still require the red warning modal if enabled

If the user confirms, the app proceeds with the cache/install task for that exact version.

## Downloads tab behavior

The selected behavior is `Active only`.

For this milestone, Downloads will show:

- queued jobs
- downloading jobs
- verifying/finalizing jobs
- cache-hit jobs while the task is still active

Successful jobs disappear once complete.

### Failure handling in Downloads

Default chosen:

- failed jobs stay visible in Downloads until the next app restart or until the next state reload that clears inactive jobs
- successful jobs disappear immediately after completion

## Cache visibility

The cache is not a primary UI surface, but users can access it from Settings.

### Settings additions

Add a new `Cache` subcategory under `Global settings`, between:

- `Warn options`
- `Danger zone`

`Cache` contains:

- cache summary row:
  - archive count
  - total size
- `Open cache folder` action

`Danger zone` gains:

- `Clear cache` action
- existing `Reset all data` action remains

## Clear cache behavior

`Clear cache`:

- deletes all cached archive files
- deletes cache rows from SQLite
- clears download/install task rows associated with this cache-only milestone
- leaves profiles intact
- leaves catalog metadata intact
- does not touch warning settings

### Active downloads restriction

If any download task is currently queued/running/verifying:

- `Clear cache` is blocked
- the UI shows a user-readable error:
  - `Cannot clear the cache while downloads are active.`

## Reset all data behavior

This milestone upgrades `Reset all data` from SQLite-only to full user-data reset for current features.

After reset, the app should be equivalent to a clean first launch for current implemented features:

- built-in `Default` profile exists
- active profile is `default`
- warning prefs reset to defaults
- cached catalog metadata removed
- cached archive files removed
- active/failed download tasks removed

## Architecture

## Backend modules

Add:

- `src-tauri/src/services/cache_service.rs`
- `src-tauri/src/services/download_service.rs`
- `src-tauri/src/commands/cache.rs`
- `src-tauri/src/commands/downloads.rs`

### Reuse existing command style

- Tauri async commands
- `spawn_blocking` for DB/file/network work
- SQLite behind `Arc<Mutex<Connection>>`

## AppState changes

Extend `src-tauri/src/app_state.rs` to create and expose cache paths:

- `app_data_dir`
- `cache_dir`
- `cache_archives_dir`
- `cache_tmp_dir`

Create:

- `$APP_DATA/cache`
- `$APP_DATA/cache/archives`
- `$APP_DATA/cache/tmp`

## Filesystem layout

```text
$APP_DATA/
  cache/
    archives/
      thunderstore/
        <version-id>.zip
    tmp/
      <task-id>.part
```

## Persistence model

## Reuse legacy table names

To stay compatible with the user’s existing local DB, reuse:

- `cached_archives`
- `install_tasks`
- `download_jobs`

### Migration strategy

Add:

- `src-tauri/migrations/0004_cache_downloads.sql`

This migration should:

- `CREATE TABLE IF NOT EXISTS` for the three tables and required indexes
- match the prior experiment’s table shapes closely
- avoid destructive schema replacement
- be paired with a repair step in `db/mod.rs` if needed

## Table definitions

### `cached_archives`

- `cache_key TEXT PRIMARY KEY`
- `source_kind TEXT NOT NULL`
- `package_id TEXT NULL REFERENCES packages(id) ON DELETE RESTRICT`
- `version_id TEXT NULL REFERENCES package_versions(id) ON DELETE RESTRICT`
- `sha256 TEXT NOT NULL`
- `archive_name TEXT NOT NULL`
- `relative_path TEXT NOT NULL`
- `file_size INTEGER NOT NULL`
- `source_url TEXT NULL`
- `first_cached_at TEXT NOT NULL`
- `last_used_at TEXT NOT NULL`

Rules:

- in this milestone, `cache_key = version_id`
- `source_kind = "thunderstore"`

### `install_tasks`

- `id TEXT PRIMARY KEY`
- `profile_id TEXT NULL`
- `kind TEXT NOT NULL` with only `cache_version`
- `status TEXT NOT NULL`
- `title TEXT NOT NULL`
- `detail TEXT NOT NULL`
- `progress_step TEXT NULL`
- `progress_current INTEGER NOT NULL`
- `progress_total INTEGER NOT NULL`
- `error_message TEXT NULL`
- `created_at TEXT NOT NULL`
- `started_at TEXT NULL`
- `finished_at TEXT NULL`

### `download_jobs`

- `id TEXT PRIMARY KEY`
- `task_id TEXT NOT NULL REFERENCES install_tasks(id) ON DELETE CASCADE`
- `package_name TEXT NOT NULL`
- `version_label TEXT NOT NULL`
- `source_kind TEXT NOT NULL`
- `status TEXT NOT NULL`
- `cache_hit INTEGER NOT NULL`
- `bytes_downloaded INTEGER NOT NULL`
- `total_bytes INTEGER NULL`
- `speed_bps INTEGER NULL`
- `progress_label TEXT NOT NULL`
- `error_message TEXT NULL`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

## Backend services

## `cache_service`

Responsibilities:

- compute archive paths for exact version ids
- look up cache rows by `version_id`
- verify cached file existence
- insert/update `cached_archives`
- clear cache files and cache DB rows
- compute cache summary:
  - archive count
  - total bytes
  - cache folder path

### Cache-hit rule

A version is treated as cached only if:

1. a `cached_archives` row exists for that `version_id`
2. the target file exists on disk

If the DB row exists but the file is missing:

- treat it as a cache miss
- remove/repair the stale row
- proceed to download

## `download_service`

Responsibilities:

- create/update install task rows
- create/update download job rows
- queue/cache one Thunderstore version
- dedupe active in-flight downloads by `version_id`
- list active downloads for the Downloads tab

### In-flight dedupe

If a request arrives for a version that already has a queued/running/verifying task:

- return the existing task id
- do not start a second network download

### Cache task flow

For `queue_install_to_cache(package_id, version_id)`:

1. validate the version exists in local catalog metadata
2. create or reuse an active task/job
3. check cache row + file existence
4. if cached:
   - update `last_used_at`
   - mark job `cached`
   - mark task succeeded
5. if not cached:
   - create temp file under `cache/tmp/<task-id>.part`
   - download from `package_versions.download_url`
   - update `bytes_downloaded` / `total_bytes` / `speed_bps`
   - compute SHA-256
   - move the finished file to `cache/archives/thunderstore/<version-id>.zip`
   - upsert `cached_archives`
   - mark task/job succeeded

## Settings/cache service behaviors

### `open_cache_folder`

Add a backend command that opens the cache folder in the platform file explorer.

Chosen implementation default:

- use a backend opener crate such as `opener`

### `clear_cache`

Behavior:

1. verify no active download tasks exist
2. delete files under `$APP_DATA/cache/archives`
3. delete temp files under `$APP_DATA/cache/tmp`
4. clear rows from:
   - `cached_archives`
   - `download_jobs`
   - `install_tasks` for `cache_version`
5. return fresh cache summary

## Frontend/API changes

## New frontend API modules

Add:

- `src/lib/api/cache.ts`
- `src/lib/api/downloads.ts`

## New/updated command surface

### Cache commands

```ts
queueInstallToCache(input: {
  packageId: string;
  versionId: string;
}): Promise<{ taskId: string }>

getCacheSummary(): Promise<{
  archiveCount: number;
  totalBytes: number;
  cachePath: string;
  hasActiveDownloads: boolean;
}>

openCacheFolder(): Promise<void>

clearCache(): Promise<{
  archiveCount: number;
  totalBytes: number;
}>
```

### Download commands

```ts
listActiveDownloads(): Promise<DownloadJobDto[]>
getTask(taskId: string): Promise<InstallTaskDto | null>
```

## Important changes to public APIs/interfaces/types

Add to `src/lib/types.ts`:

- `CacheSummaryDto`
- `InstallTaskDto`
- `DownloadJobDto`

The current mock `DownloadItem` should be replaced by backend DTO-backed state for this milestone.

## Frontend store refactor

Update `src/lib/store.ts`.

### New backend-backed state

- `activeDownloads: DownloadJobDto[]`
- `cacheSummary?: CacheSummaryDto`
- `isLoadingDownloads: boolean`
- `isLoadingCacheSummary: boolean`
- `downloadError: string | null`
- `cacheError: string | null`
- `activeCacheTaskIds: string[]`

### Install action behavior

Replace the placeholder `installVersion()` behavior with:

1. version warning gate remains the same
2. confirmed action calls `queueInstallToCache(packageId, versionId)`
3. start polling active downloads while any active task exists
4. refresh cache summary when task completes
5. append activity messages for:
   - cached hit
   - downloaded to cache
   - failure

### Polling model

Use polling for this milestone:

- start polling every 500ms when a task is queued
- stop polling when no active downloads remain

## UI changes

## Browse / Package detail

Keep the visible button labels:

- `Install`
- `Install version`

Behavior change only:

- they now trigger real cache/download work

## Downloads tab

Replace the mock `downloads` list with backend-backed active download jobs.

Display:

- package name
- version
- status pill
- progress label
- bytes / speed when downloading
- `cache hit` wording for cached fast path

### Empty state

When no active jobs exist:

- show `No active downloads.`

## Settings screen

Add a `Cache` subcategory under `Global settings`.

### `Cache` section contents

1. summary row:
   - `Cached archives`
   - count and size
2. `Open cache folder` row:
   - button opens the folder in the system file explorer

### `Danger zone` additions

Add:

- `Clear cache`
- confirmation required
- button disabled while active downloads exist

## Failure modes and handling

### Missing package/version metadata

- fail with user-readable error

### Missing `download_url`

- fail with `Version cannot be downloaded from local metadata. Refresh the catalog and try again.`

### Stale cache row

- delete stale row
- treat as cache miss

### Interrupted/partial download

- overwrite temp file on next attempt
- delete temp file best-effort on failure

### Duplicate clicks

- if the same version is already actively downloading, return the existing task id

### Clear cache during active downloads

- block operation
- return clear error

### Reset all data

- if cache deletion fails, reset fails loudly rather than claiming success

## Testing and verification

## Rust/unit scenarios

- cache miss when no DB row exists
- cache miss when DB row exists but file is missing
- cache hit when row and file exist
- successful download writes temp file then final archive
- SHA-256 is computed and stored
- repeated queue request for the same version while active returns the same task id
- clear cache clears archive files and rows when idle
- clear cache fails when active downloads exist
- reset clears cache directory and reseeds `Default`

## Integration/manual scenarios

- install one uncached version from Browse, see it appear in Downloads, then disappear on success
- install the same version again, verify cache-hit path instead of network download
- verify Settings cache summary updates after install
- open cache folder from Settings and confirm OS file explorer opens
- clear cache from Settings and verify files are gone
- after clear cache, reinstall same version and verify network download happens again
- red/broken warning flow still gates before the cache action
- repeated Install clicks on the same version do not start duplicate downloads
- `Reset all data` clears profiles, warning prefs, cached catalog metadata, and cache files

## Acceptance criteria

This milestone is done when:

1. `Install` and `Install version` perform real cache-aware backend work.
2. Exact-version cache hits avoid redownloading.
3. Cache misses download Thunderstore archives into app data.
4. Downloads tab shows active real download/cache tasks.
5. Settings can open the cache folder.
6. Settings can clear the cache safely.
7. `Reset all data` now clears cache files too.
8. No profile/modpack install state is changed yet.
9. Existing warning flows still gate red/broken versions.
10. `npm run build` passes.
11. `cargo check --manifest-path src-tauri/Cargo.toml` passes.

## Assumptions and defaults

- this is a `cache only` milestone, not profile install
- Browse buttons remain labeled `Install` / `Install version`
- Downloads is `active only`
- failed jobs remain visible until restart or a later active-state reload clears them
- cache storage is global and exact-version keyed by `version_id`
- the backend uses local catalog metadata as the source of truth for `download_url`
- the implementation must remain compatible with the existing local DB, which still contains legacy cache/install tables from the reverted experiment
