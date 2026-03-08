# Current Stage Notes

Last updated: 2026-03-08

This file is the short-term implementation handoff for the current stage of the project.
The broad product plan remains in [plan-v1.md](./plan-v1.md).

## Current Project State

- Project root: `/home/crast/dev/49modman`
- Git repo is initialized locally on branch `main`
- Current recorded baseline commit: `3826f97` (`Initial frontend and backend scaffold`)
- The app now runs as a real Tauri desktop window instead of only as a browser/Vite page
- The current frontend is still visually close to the browser prototype, but it is now being exercised through the desktop app runtime
- The install/cache/modpack experiment was reverted
- The profile-only milestone is implemented
- The cache-only milestone is now implemented and working
- Browse install now performs real profile installs:
  - cache hit or cache download first
  - extract zip into active profile `mods/`
  - update profile `manifest.json` `mods[]`
- Overview now renders installed mods from profile manifest data
- Overview installed rows now load `icon.png` from each installed mod folder when available
- uninstall safety now warns before removing mods that other installed mods depend on:
  - shows dependant mod list in a confirmation modal
  - modal can disable future prompts (`Do not show this again`)
  - Settings `Warn options` now includes `Warn on uninstall with dependants`
- Overview dependency-warning icon experiment was removed after inconsistent behavior reports:
  - no missing-dependency warning icon is currently shown in Overview rows
  - tracked in [overview-missing-dependency-warning-issue.md](./overview-missing-dependency-warning-issue.md)
- Reset-all-data UX/backend flow has been hardened:
  - schema-safe reset for legacy tables
  - visible progress modal during reset
  - automatic Browse recache/refresh after reset completes
- Browse card layout has been hardened for narrow windows:
  - list/card containers now enforce shrink bounds (`min-width: 0`)
  - long card text/chips now wrap instead of forcing horizontal overflow
  - prevents package cards from being cut off on the right after startup or Browse refresh
- Browse sorting/discovery controls are now expanded:
  - Browse toolbar now includes a sort dropdown with:
    - `Most downloads` (default)
    - `Compatibility`
    - `Last updated`
    - `A-Z`
    - `Z-A`
  - `Most downloads` now sorts by summed per-version downloads from cached catalog version rows
  - package cards now show download count inline after author text:
    - `by {author} • {downloads} downloads`
- Browse first-page refresh now uses a blocking list loading overlay when cards are already visible:
  - overlay uses the same lock card + spinner visual pattern as the package detail lock state
  - card list scroll and interaction are blocked while the overlay is active
  - overlay is anchored to the visible list viewport (not the scrolled card content)
- app now disables the built-in right-click context menu globally
- Browse install controls now reflect manifest-installed state in the active profile:
  - package-card quick action becomes red `Uninstall` with a trash icon when that package is already installed
  - detail-panel main action becomes red `Uninstall` with a trash icon when that package is already installed
  - version-row action for the installed version becomes red `Uninstall version` with a trash icon
  - other version-row actions become `Switch version` and replace currently installed version(s) of that package
  - switch-version flow preserves red/broken warning modal behavior before proceeding
  - Browse empty/loading status panels now include additional padding for readability
  - while a package install/uninstall/switch operation is in progress:
    - that package card action button is disabled, grey, and shows spinner-only
    - if the same package is selected in detail, the detail panel is locked with `Waiting...`
- Browse install/switch actions now support dependency-aware installation:
  - default install/switch queues the selected version and then resolves/queues dependencies from cached catalog metadata
  - dependency actions are manifest-aware:
    - skip if exact dependency package+version is already installed
    - if same package is installed at a different version, switch only when required version is higher
    - skip lower/equal required versions for already-installed packages
    - install dependency when package is not installed
- Browse detail install/switch actions now expose an install-mode dropdown (excluding uninstall actions):
  - `Install without dependencies` skips dependency installs for that action
  - this path uses a confirmation prompt when enabled by warning settings
  - Settings `Warn options` now include `Warn on install without dependencies`
- Settings/Overview follow-up polish is now implemented:
  - Settings size summaries now display human-readable disk units (`B`, `KiB`, `MiB`, etc.) for:
    - `Cached archives`
    - both size counts in `Profile storage`
  - `Clear cache` moved out of `Danger zone` and into the `Cache` subcategory as the final cache setting row
  - `Clear cache` retains danger styling (red header emphasis) while remaining in Cache
  - Overview installed section heading now shows dynamic count text with singular/plural:
    - `{n} installed mod`
    - `{n} installed mods`
  - each Overview installed-mod row now includes `Jump to details` (external-link icon):
    - switches to Browse
    - opens the package detail panel
    - focuses/highlights the exact installed version row
    - auto-enables that version's status filter if currently hidden so the target row is visible
- Launch system milestones `L0` through `L5` are now implemented and wired:
  - backend launch preflight, staging, activation/deactivation, launch execution, diagnostics
  - Linux Proton runtime discovery/selection for direct launch
  - UI launch feedback panel with diagnostics/open and activation repair actions
- Launch hardening fixes are now in place:
  - runtime stage path normalization strips package wrapper roots (for example `BepInExPack/`)
  - top-level `plugins/patchers/config/core` payloads are remapped into `BepInEx/...`
  - `.lethalbundle` payloads from non-anchor roots are remapped into `BepInEx/plugins/...` so LethalLevelLoader can discover custom levels
  - stale activation cleanup now treats remaining managed files as blocking, while retained non-empty managed directories no longer block re-activation
  - Linux direct launch now sets `WINEDLLOVERRIDES=winhttp=n,b` for Doorstop/BepInEx injection
  - Linux modded Steam launch now validates Steam per-game launch options before activation and fails with actionable guidance when `%command%` + `WINEDLLOVERRIDES=winhttp=n,b` are missing
  - Steam `localconfig.vdf` parsing now handles escaped quotes in launch option values (for example `WINEDLLOVERRIDES=\"winhttp=n,b\" %command%`)
  - launch feedback panel layout was fixed so warning/success panels no longer consume the full content area; panel padding and heading spacing were tightened for readability
  - launch success icon mapping was corrected from an invalid key to a valid icon token
  - Linux Steam launch-option validation now accepts any launch-options value that contains both `%command%` and `winhttp=n,b`, instead of requiring an exact string match
  - Steam launch-option error copy now uses explicit step-by-step wording for casual Steam/Proton users
- Launch/settings UX follow-up polish is now in place:
  - `Preferred Proton runtime` is now grouped under Settings -> `Launch (Linux)`
  - topbar launch buttons now append `(Direct)` only for direct mode
  - topbar launch buttons no longer append `(Steam)` in steam mode
- Launch/runtime guard + resource-saver follow-up is now in place:
  - launch now rejects duplicate attempts while a launch is in progress
  - launch now rejects when Lethal Company is already running, with stale tracked-PID cleanup before blocking
  - Settings -> `Performance` now includes `Conserve resources while game is running` (default off)
  - while resource saver is active:
    - heavy Browse/reference loading and navigation actions are blocked
    - Browse/reference in-memory lists are cleared
    - backend dependency index cache is invalidated
    - SQLite memory trim is requested (`PRAGMA optimize; PRAGMA shrink_memory;`)
    - Linux allocator trim is requested (`malloc_trim(0)`) as best effort
  - when resource saver exits, dependency index prewarm is restored automatically
  - app now polls launch runtime state so resource saver can auto-enter on game start and auto-exit on game close
  - Settings -> `Performance` now includes `View RAM usage` modal:
    - auto-refreshes every 2 seconds while open
    - shows process-level RAM totals and per-process rows for the app process tree
    - Linux reports RSS/PSS/private/shared/swap from `/proc/*/smaps_rollup` when available
    - Windows reports working-set (RSS-style) process rows via PowerShell/CIM, with unavailable fields noted
- Cache follow-up is now in place:
  - Settings -> `Cache` now includes `Clear unreferenced cache` alongside full `Clear cache`
  - unreferenced cleanup has a confirmation modal that previews exactly which cached mod versions will be removed
  - preview includes package/version rows and per-version size
  - cleanup preserves versions installed in any profile, including disabled installed mods
  - cleanup removes only unreferenced cache entries (and their cache-task rows), not the entire cache
- `.49pack` profile-pack flows are now implemented from Profiles:
  - `Export .49pack` is available on Profiles tab and writes a ZIP pack containing:
    - `manifest.json`
    - `profile.json`
    - `mods.lock.json`
    - optional `notes.txt`
    - installed profile mod payload directories (`mods/...`)
    - profile runtime config/plugins payload (`config/BepInEx/...`) when present
  - `Import .49pack` is now a 2-step flow:
    1. pick file + preview
    2. confirm import
  - preview modal lists all mods that will be imported
  - preview modal supports `Do not show this again`
  - warn preference is persisted as `Warn on profile import` in Settings -> `Warn options`
  - profile import creates a new profile (name conflict-safe), imports payloads, and restores manifest mod entries
- local folder-open actions no longer block the app event loop while the file explorer window is open:
  - `Open profiles folder`
  - `Open active profile folder`
  - `Open cache folder`
  - opener commands now launch via detached child process (`spawn`) and are reaped in a background thread
- Overview now includes an `Import mod` button (upload icon) near the installed-mod section header:
  - opens a `.zip` file picker
  - extracts selected archive into active profile `mods/`
  - upserts installed mod entry into profile `manifest.json`
  - imported entries use `sourceKind: "local_zip"`
  - local `.zip` cache copy features are currently disabled in frontend UX:
    - import is one-click from picker to profile import
    - frontend always sends `addToCache: false`
    - backend cache-support code remains available but is not exposed
- Thunderstore version icons are now surfaced in Browse:
  - backend now parses version-level `icon` from `/packages/` payloads
  - `package_versions` now persists `icon_url` and repairs legacy DBs by adding the column at runtime when missing
  - Browse package cards now use the version icon URL when available, with existing fallback behavior
- launch precheck dependency UX is now more forgiving for catalog mismatch cases:
  - `PROFILE_DEPENDENCY_STATE_INVALID` now renders a casual-user explanation
  - feedback panel exposes `Run anyway` only for that specific error
  - `Run anyway` retries modded launch with dependency validation explicitly skipped
- dependency precheck now excludes local imports:
  - validation applies only to enabled mods with `sourceKind: "thunderstore"`
  - enabled local `.zip` imports are ignored for dependency-state validation
- Settings main content now has a dedicated scroll container so long Settings pages are fully reachable in smaller windows
- Browse detail panel content below category chips is now split into tabs:
  - `Details`: README content when available
  - `Versions`: existing versions/downloads/actions content
- Browse detail README source now uses latest Thunderstore-version metadata:
  - backend resolves latest `website_url` for the selected package from Thunderstore API data
  - when `website_url` is a GitHub repo, README is requested from GitHub `/readme`
  - README fetch now uses GitHub-rendered HTML (`application/vnd.github.html+json`) to preserve:
    - raw inline HTML blocks (for example `<p align="center"><img ...>`)
    - table rendering
    - standard GitHub-flavored markdown formatting
  - if any check/fetch fails, `Details` is hidden and `Versions` stays focused
- Browse detail tab UX was polished to avoid late tab switching:
  - selecting a mod now enters a short resolving state before showing tab content
  - this prevents showing `Versions` first and then jumping to `Details`
  - the `Loading package details...` card is centered in the same content area used by `Details`/`Versions`
- Settings now supports relocating storage paths for both cache and profiles:
  - new Settings rows show current `Cache location` and `Profiles location`
  - each row has `Move` action (folder picker)
  - migration copies data to the selected path with a blocking progress modal + progress bar
  - migration requires destination folders to be empty (or missing)
  - on successful copy, old folders are deleted, new paths are persisted in settings, and the app restarts

## Next Milestone

Implemented from:

- [profile-system-m1.md](./profile-system-m1.md)

Current state:

- real backend-backed profiles in SQLite
- built-in undeletable `Default` profile
- active profile persisted in `settings["profiles.active_id"]`
- Profiles tab supports:
  - real create
  - real delete
  - real active-profile switching
- Overview now uses the real active profile and shows manifest-backed installed mods
- Settings now includes:
  - `Warn options`
  - `Cache`
  - `Danger zone`
  - `Reset all data`
- Browse `Install` / `Install version` now queue real cache tasks
- Browse package cards now expose a quick-install button for the recommended version
- Downloads now shows real active cache/download work
- cache is global, exact-version, and stored in app data
- profile storage folders are now scaffolded under app data and reconciled at startup:
  - `$APP_DATA/profiles/<profile_id>/manifest.json`
  - `$APP_DATA/profiles/<profile_id>/mods/`
  - `$APP_DATA/profiles/<profile_id>/runtime/BepInEx/plugins/`
  - `$APP_DATA/profiles/<profile_id>/runtime/BepInEx/config/`
- profile manifests are now maintained as schema v1 metadata + installed `mods` entries
- deleting a profile now also deletes its profile folder on disk
- Settings now includes a `Profiles` subcategory with:
  - `Open profiles folder`
  - `Open active profile folder`
  - profile storage summary row:
    - profile count
    - total profiles-folder size
    - active profile-folder size
- Profiles tab cards now show each profile's own storage size

Last completed milestone:

- [cache-system-m1.md](./cache-system-m1.md)

Current planned milestone:

- post-install-state polish and follow-up UX/backend hardening

Current locked behavior:

- install scope is now `cache + active profile`
- Browse detail `Install` now includes the chosen version label
- Downloads is still `active only`
- Overview installed rows now support real mod-state controls:
  - enable/disable toggle (persists `mods[].enabled` in manifest)
  - uninstall (removes profile mod folder + removes manifest entry)

## Current Uncommitted Work

This checkpoint captures local `.zip` import, Browse version icons, launch dependency precheck/run-anyway follow-ups, storage relocation with progress + restart, and non-blocking folder opener fixes.

## Profile Milestone Notes

- `Default` uses fixed id `default`
- `Default` cannot be deleted
- creating a profile makes it active immediately
- deleting the active non-default profile falls back to `Default`
- `reset_all_data` now resets both:
  - cache files on disk
  - SQLite-backed user state:
  - profiles
  - settings
  - reference overrides
  - cached catalog metadata rows
  - sync state
- reset flow now tolerates legacy DB history by dropping obsolete tables if present before reset seeding:
  - `profile_mod_dependencies`
  - `profile_mods`
  - `local_mods`
- reset flow now shows a blocking progress modal in Settings after confirmation:
  1. delete local app data
  2. restore default profile/settings
  3. refresh Browse data from Thunderstore
  4. finalize and return to normal UI
- Browse installs are now real and modify active profile state
- local mod import from `.zip` is now implemented from Overview and writes manifest entries as `sourceKind: "local_zip"`
- `.49pack` profile export/import is now implemented for profile metadata + manifest/payload import/export

## Profile Storage And Manifest Notes (Post Install Activation)

- profile folders are keyed by profile id, not profile name
- profile storage is ensured in these flows:
  - app startup (reconciliation across all profiles)
  - profile create
  - profile update
  - opening active profile folder
- current scaffolded runtime path is:
  - `runtime/BepInEx/plugins`
  - `runtime/BepInEx/config`
- `manifest.json` is rewritten atomically for profile metadata updates
- manifest `mods[]` now persists installed Thunderstore versions for each profile
- installed mod entries include:
  - package/version identity
  - enabled flag (persisted, user-toggleable)
  - source kind (`thunderstore` for Browse installs, `local_zip` for Overview `.zip` imports)
  - install directory under `mods/`
  - installed timestamp
- manifest read APIs now enrich installed-mod DTOs with optional `iconDataUrl` if `icon.png` exists in that mod folder
- manifest reads now also reconcile stale entries by pruning `mods[]` rows whose `installDir` no longer exists on disk
- Overview `.zip` import currently does not expose cache-copy UI; imports run as profile-only (`addToCache: false`) while backend local-cache paths remain dormant for future re-enable
- `reset_all_data` now clears profile folders and then reseeds + re-ensures `default` profile storage
- per-profile storage size is computed from profile directory bytes and returned in `list_profiles`
- Settings profile summary is returned via backend command:
  - `get_profiles_storage_summary`

## Cache + Profile Install Notes

- this is the first real install-adjacent filesystem work since the revert
- install scope is now `cache + active profile extract`
- Downloads behavior is `active only`
- Browse labels now adapt to installed state:
  - `Install {version}` when the package is not installed
  - `Uninstall` on package-level actions when any version of that package is installed
  - `Uninstall version` on the exact installed version row
  - `Switch version` on other version rows when a package version is already installed
- install flow now:
  1. check exact version in shared cache
  2. download to cache on miss
  3. extract cached zip into active profile `mods/<package>-<version>/`
  4. upsert manifest entry for that exact package/version
- repeated installs of the same exact version currently re-extract into the same target folder and refresh manifest timestamp
- multiple versions of the same package can still exist, but Browse switch-version actions now remove currently installed version(s) of the selected package before installing the target version
- uninstall and enable/disable are now implemented in Overview installed rows
- the current local SQLite DB still contains legacy tables from the reverted experiment:
  - `cached_archives`
  - `install_tasks`
  - `download_jobs`
  - `profile_mods`
  - `profile_mod_dependencies`
  - `local_mods`
- cache implementation was written to stay compatible with that DB shape rather than assuming a clean slate

## Browse Responsiveness Fix Notes

- a lock-contention regression was found after profile-install activation:
  - Browse detail and quick-install actions could appear non-responsive while install worker held the SQLite mutex during profile file extraction/manifest updates
- fix applied:
  - reduce DB lock lifetime in install worker
  - perform filesystem-heavy profile extraction outside long DB lock scope
  - reduce post-install frontend refresh work to active-profile refresh instead of full profile/storage refresh
- result:
  - Browse detail selection and quick-install interactions remain responsive while install jobs complete

## Browse Install Polish Notes

- the detail-panel `Install` button now derives its target version from the shared recommendation logic
- the detail-panel `Install` button color now follows the exact chosen version status and updates reactively when the selected package changes
- the detail-panel `Install` button label now includes the selected version:
  - `Install {version}`
- package-card quick install now exists in the bottom-right corner of each Browse row:
  - icon-only
  - same target version as the detail-panel Install button
  - same warning/install flow as the detail-panel Install button
- recommendation tie-breaking is now:
  1. effective status bucket
  2. semantic version number
  3. published date
  4. downloads
- this avoids arbitrary same-day selection when multiple versions share the same publish date
- browse card data now includes the recommended version id so the list-row quick install can target the exact version directly
- install-start failures now surface more context:
  - clearer frontend error text
  - failed Downloads rows show backend error messages
- Browse now tracks per-package busy state so install/uninstall actions lock only the relevant package card and selected package detail panel
- install-related download icons now force white fill for consistency across status-colored install buttons:
  - Browse card quick-install button
  - detail `Install {version}` button
  - detail `Install version` buttons
- busy card action now uses the shared inline loading spinner element:
  - `<div class="loading-spinner" aria-hidden="true"></div>`
- default Browse install/switch now resolves dependencies and applies manifest-aware dependency actions:
  - skip already-installed exact dependency versions
  - switch dependency package versions only when required dependency version is higher than installed
  - skip lower/equal required dependency versions for already-installed packages
  - queue installs for missing dependency packages
- detail-panel install/switch buttons now include an install-mode dropdown for dependency-bearing targets:
  - `Install without dependencies` option
  - uninstall actions intentionally do not include this option
  - warning prompt for this path can be toggled via `warning.install_without_dependencies`
- desktop `get_package_detail` now includes parsed per-version dependency arrays from `dependencies_json`, which the detail-panel install-mode UI uses to decide when to show dependency install options
- install-mode triggers now use the dedicated `down-arrow-small.svg` icon and render as merged split buttons with the adjacent install action when dependency options are available
- split-button sizing now uses a shared explicit height so both halves match exactly instead of relying on per-button padding heuristics
- uninstall-related Browse actions now use a dedicated trash icon:
  - package-card `Uninstall`
  - detail-panel `Uninstall`
  - version-row `Uninstall version`
- topbar `Launch modded` play icon now also uses the same white icon override
- the detail-panel category chip row now has slightly more vertical spacing below the primary action row

## Desktop Runtime Status

The desktop runtime milestone is effectively working.

Implemented:

- Local Tauri CLI is installed via `package.json`
- Tauri runtime detection is explicit in `src/lib/runtime.ts`
- `src/lib/api/client.ts` now prefers the real Tauri backend in desktop runtime and does not silently fall back to mocks after desktop backend errors
- `src/lib/store.ts` tracks runtime kind and desktop-specific bootstrap errors
- `src/App.svelte` shows a compact desktop error panel when needed
- `vite.config.ts` and `src-tauri/tauri.conf.json` are aligned to `127.0.0.1:4173`
- `scripts/tauri-runner.mjs` wraps Tauri dev/build and applies Linux-specific environment fixes

Linux/Tauri notes:

- The app previously showed a transparent/white window
- A GBM/WebKit rendering issue was observed
- The workaround now lives in `scripts/tauri-runner.mjs`
- Relevant environment handling in the runner:
  - prepends `~/.cargo/bin` to `PATH`
  - sets `WEBKIT_DISABLE_DMABUF_RENDERER=1`
  - sets `WEBKIT_DISABLE_COMPOSITING_MODE=1`
  - sets `GDK_BACKEND=x11` if unset
  - sets `WINIT_UNIX_BACKEND=x11` if unset
- After that change, the user confirmed the app window rendered successfully
- Dev-only restart caveat for storage migration:
  - the migration flow calls `AppHandle::restart()`
  - in `npm run tauri:dev`, the app UI is served from `http://127.0.0.1:4173` (`devUrl`)
  - after restart in dev mode, the webview can briefly fail with `127.0.0.1: Connection refused` and require rerunning `npm run tauri:dev`
  - this is expected to be a dev-runtime behavior; release bundles load from `frontendDist` and do not depend on the Vite dev server URL

## Last Verified Commands

These were already verified successfully before these notes were written:

- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm exec tauri -- --version`
- `npm run tauri:dev`

## Current Product/Engineering Focus

The current focus has shifted from cache-only install activation to active-profile install correctness and installed-mod UX polish in Overview.

Dependency-view notes below are preserved as historical context from the previously completed focus.

Current dependency-view state:

- `View dependencies` now exists in the version context menu in Browse package detail
- the app now has a real dependency modal backed by:
  - `src-tauri/src/commands/dependencies.rs`
  - `src-tauri/src/services/dependency_service.rs`
  - `src/lib/api/dependencies.ts`
  - `src/components/DependencyModal.svelte`
- dependency resolution is local-catalog only and now uses an app-lifetime in-memory dependency index cache
- dependency index cache warmup now exists as a dedicated backend command:
  - `warm_dependency_index`
- dependency index cache is invalidated after:
  - catalog sync writes
  - reference override writes
  - reset-all-data
- dependency index build now parses version dependency arrays lazily per visited node (instead of eagerly parsing all versions on every request)
- modal defaults to `Summary` and keeps `Tree` as the advanced inspection view
- Summary sections are:
  - `Direct`
  - `Indirect`
  - `Unresolved`
- Summary deduplicates resolved entries by package and keeps the highest required version as the primary row target
- when one package has multiple required versions, lower versions are preserved in the same Summary row as strikethrough metadata
- Tree remains exact-version and still uses:
  - `Resolved`
  - `Unresolved`
  - `Cycle`
  - `Repeated`
- repeated exact-version tree branches are compacted to lightweight reference rows
- lower-version rows in Tree are visually de-emphasized (strikethrough/italic version + italic title + muted row background)
- resolved entries can jump to the exact dependency version in Browse
- exact target version rows still scroll into view and highlight temporarily

Browse startup / interaction behavior tied to dependency optimization:

- startup overlay now includes a `Prepare dependencies` step
- app startup with an existing catalog now keeps the overlay visible until:
  1. first Browse page is loaded
  2. selected package detail is loaded
  3. dependency index warmup completes
  4. a non-force catalog freshness check completes
- if startup freshness check returns `synced`, Browse first page/detail are reloaded while the overlay is still visible, then dependency warmup runs again against updated metadata
- there is no longer a background post-overlay refresh in cached-catalog startup; this avoids the confusing `Refreshing` phase after the app appears interactive
- during first-page Browse searches, the right-side package detail panel is now interaction-locked to avoid stale-detail discrepancies while results are being replaced

Current known gaps / follow-up candidates:

- if dependency graph performance is still poor on very large graphs, the next follow-up is a normalized dependency edge table populated at catalog sync time
- tree-level lower-version styling is currently visual-only and does not change exact dependency-path semantics

Planning docs for dependency work:

- implemented first pass: [dependency-view-m1.md](./dependency-view-m1.md)
- implemented v1.1 pass: [dependency-view-v1-1.md](./dependency-view-v1-1.md)

## Confirmed Performance Bottlenecks Before The Fix

### Frontend

In `src/components/BrowseScreen.svelte`:

- the full package list is rendered eagerly
- there is no pagination
- there is no virtualization
- there is no infinite-scroll sentinel

In `src/lib/store.ts`:

- Browse state is still treated as one full result set
- there is no cursor/page model
- there is no distinction between:
  - first blocking load
  - background refresh
  - next-page fetch

### Backend

In `src-tauri/src/services/catalog_service.rs`:

- `search_packages()` currently loads the full package record set
- that path hydrates all packages and all versions before filtering/sorting
- the current structure is effectively a full-catalog hydration path plus N+1 version queries
- `get_package_detail()` also routes through the broad package-record loader instead of querying a single package directly

That meant the performance problem was both:

- backend query shape
- frontend full-list rendering

## Browse Performance Work Now Implemented

### Backend

Implemented in `src-tauri/src/services/catalog_service.rs`:

- `search_packages` now returns a paginated result object instead of a full array
- package-card browse results are now computed in SQLite with a window-function query
- card recommendation/effective-status selection no longer requires hydrating every package/version into Rust on each search
- `get_package_detail` now loads one package directly instead of routing through the full package-record loader

New backend result shape:

- `items`
- `nextCursor`
- `hasMore`
- `pageSize`

Related backend changes:

- `src-tauri/src/commands/catalog.rs` now returns the paginated search result type
- `src-tauri/migrations/0002_catalog_indexes.sql` was added for package/reference lookup indexes
- `src-tauri/src/db/mod.rs` now runs both migrations

### Frontend state and API

Implemented:

- `src/lib/types.ts` now includes the paginated search result shape plus Browse paging/loading state fields
- `src/lib/api/catalog.ts` now expects paginated search results
- `src/lib/api/mock-backend.ts` now mirrors the paginated API contract
- `src/lib/store.ts` now has distinct flows for:
  - first page load
  - next page load
  - refresh
  - initial blocking overlay

The store now tracks:

- `catalogNextCursor`
- `catalogHasMore`
- `catalogPageSize`
- `isLoadingCatalogFirstPage`
- `isLoadingCatalogNextPage`
- `isCatalogOverlayVisible`
- `catalogOverlayMessage`

### Browse UI

Implemented in `src/components/BrowseScreen.svelte`:

- infinite scroll via `IntersectionObserver`
- explicit first-page loading state
- explicit next-page loading state
- empty state
- retry/error state
- end-of-results state

Implemented in `src/App.svelte` and `src/app.css`:

- whole-app loading overlay for initial blocking catalog retrieval

Overlay behavior now:

- shown only when there is no cached catalog and the first blocking retrieval is happening
- not shown for background refreshes or next-page fetches

### Verification

Verified after implementation:

- `npm run build`
- `~/.cargo/bin/cargo check --manifest-path src-tauri/Cargo.toml`

## Follow-up Fixes After First Browse Optimization Pass

The first performance pass exposed three immediate follow-up issues:

- manual `Refresh` still felt like it froze the app
- the loading overlay was not reliably visible during refresh work
- the Settings reference library was still using the old unoptimized backend path

Those have now been addressed.

### Desktop command execution

Updated:

- `src-tauri/src/app_state.rs`
- `src-tauri/src/commands/catalog.rs`
- `src-tauri/src/commands/reference.rs`
- `src-tauri/src/commands/settings.rs`

Current behavior:

- app state now uses `Arc<Mutex<Connection>>`
- catalog/reference/settings commands are now async Tauri commands
- expensive command work is dispatched through `tauri::async_runtime::spawn_blocking`
- this is intended to prevent the desktop window from appearing frozen while large sync/query work runs

### Refresh overlay

Updated:

- `src/lib/store.ts`
- `src/App.svelte`
- `src/app.css`

Current behavior:

- manual Refresh now uses the full-app overlay too
- the store waits one paint frame before starting the blocking refresh path so the overlay can actually render
- the overlay now shows simple step indicators:
  - Contact Thunderstore
  - Update local cache
  - Load Browse results

### Settings reference library backend path

Updated:

- `src-tauri/src/services/reference_service.rs`

Current behavior:

- reference rows are now loaded through a direct SQL query
- the service no longer routes through the old full package-record hydration path
- single-row lookup after reference updates is also direct now

### Remaining Settings risk

The Settings reference library backend is now materially cheaper than before, but the frontend still renders the returned rows as one list.
If that screen still feels heavy on very large result sets, the next step would be pagination or infinite scroll for Settings too.

## Second Performance Pass

The first follow-up still left three problems in practice:

- entering Settings could still freeze the app and spike RAM
- Browse infinite scroll was unreliable
- Browse did not clearly indicate when a new search or additional page load was in progress

Those have now been addressed.

### Settings lazy loading and pagination

Implemented:

- `src-tauri/src/services/reference_service.rs` now returns paginated reference results
- `src-tauri/src/commands/reference.rs` now accepts paginated list input and returns a paginated result
- `src/lib/api/reference.ts` and `src/lib/api/mock-backend.ts` match the paginated contract
- `src/lib/store.ts` no longer preloads the reference library during app bootstrap
- Settings reference rows are now loaded only when the Settings view is opened
- reference rows now page in incrementally instead of loading one giant result set

Important consequence:

- the Settings screen should no longer build the full reference library in memory up front

### Browse infinite scroll reliability

Implemented:

- `src/components/BrowseScreen.svelte` no longer relies on the previous `IntersectionObserver` sentinel path
- Browse now uses a direct scroll-threshold trigger
- Browse also auto-loads more pages if the current page does not fill the scroll container

### Explicit search/loading indicators

Implemented:

- Browse now shows:
  - `Searching cached mods...`
  - `Loading more mods...`
- Settings now shows:
  - `Searching reference rows...`
  - `Loading more reference rows...`

### Additional Browse responsiveness improvement

Implemented:

- `src/lib/store.ts` no longer waits for package-detail fetch completion before letting the first-page Browse result load resolve
- the package list can paint first, and the detail panel catches up asynchronously

## Browse Version Marking UX Change

The old inline reference-mark buttons on each Browse version row have been replaced with a compact overflow/context menu.

Implemented:

- `src/lib/icons.ts` now includes `three-dots-vertical`
- `src/components/PackageDetail.svelte` now supports:
  - right-click on a version row
  - overflow button on a version row
  - shared popup menu actions:
    - `View in browser`
    - `Mark as verified`
    - `Mark as broken`
    - `Clear mark`

The old inline version-row buttons were removed:

- `Verified`
- `Broken`
- `Clear`

The `Install` button remains visible on the row.

## Package Browser Link Change

The earlier `View in browser` action inside the version-row menu was replaced.

Current behavior:

- `View mod` now lives on the package detail header
- it opens the package page externally
- version-row overflow/right-click menus are now only for local mark actions

Implementation details:

- `src/components/PackageDetail.svelte` owns the package-level button
- `src/lib/api/system.ts` provides the frontend helper
- `src-tauri/src/commands/system.rs` opens the external URL through the backend using `webbrowser`

This replaced the earlier attempt to open version-specific URLs directly from the version menu.

## Settings UI Change

The Settings screen no longer renders the reference-library UI.

Important detail:

- the underlying reference-library code paths were intentionally kept intact
- only the Settings UI surface for that feature was removed
- version marking is now meant to happen directly from Browse

## Remaining Known Limits / Next Tuning Targets

The biggest Browse bottleneck was addressed, but there are still follow-up areas worth remembering:

- `reference_service.rs` still uses `load_package_records(connection)?`, which is still a full-catalog hydration path
- package detail still loads all versions for the selected package, which is acceptable for now
- there is no list virtualization yet; the app now relies on backend paging + infinite scroll
- real-world tuning may still adjust page size away from the current default of `40`
- the current overlay/loading flow needs real desktop runtime smoke-testing against the full live Thunderstore dataset

## Original Implementation Plan For This Stage

This is preserved here for reference.

### Overview

Optimize Browse using backend pagination plus frontend infinite scroll, and add a whole-app loading overlay for the initial blocking catalog retrieval.

### Core decisions

- Use backend pagination
- Use frontend infinite scroll
- Keep search submit-only
- Do not keep one giant package list in memory
- Use a whole-app overlay only for the initial blocking catalog retrieval / empty-cache case
- Use smaller local loading states for later refreshes and next-page fetches

### Backend changes

Refactor Browse backend reads so they no longer hydrate the full catalog for every search/detail request.

Planned backend work:

- change `search_packages` to return a paginated result object instead of one full array
- add a dedicated package-card page query path
- add a package-specific detail query path
- stop using the broad full-catalog loader for Browse search/detail
- use offset pagination first for simplicity
- default page size: `40`

Planned `searchPackages` result shape:

- `items`
- `nextCursor`
- `hasMore`
- `pageSize`

### Frontend changes

Replace the full-list Browse state with paginated Browse state.

Planned store actions:

- `loadBrowseFirstPage()`
- `loadBrowseNextPage()`
- `refreshBrowseList()`
- `resetBrowseList()`

Planned UI behavior:

- first page loads normally
- more pages load as the user nears the bottom of the package list
- a new submitted search resets the list to page 1
- a visible-status filter change resets the list to page 1
- package detail stays separate from package-list pagination

### Infinite scroll mechanics

Use an `IntersectionObserver` sentinel at the bottom of the package list.

Expected behavior:

- load before the user fully reaches the bottom
- prevent duplicate next-page fetches while one is already in flight

### Loading overlay

Add a full-app loading overlay in `src/App.svelte`.

Show it only when:

- there is no cached catalog
- the app is doing the first blocking sync/retrieval

Do not show the full overlay for:

- normal page fetches
- background refreshes with cached data
- next-page infinite-scroll fetches

Planned overlay copy:

- `Retrieving Thunderstore catalog...`
- `Building local cache for Browse`

### Error handling

Planned behavior:

- first load with no cache:
  - blocking overlay
  - if sync fails, show a real blocking error state
- refresh failure with cached data:
  - keep existing list visible
  - show a non-blocking error
- next-page failure:
  - keep current items visible
  - show a retry affordance near the list bottom

## Planned Type/API Changes

### Frontend

`SearchPackagesInput` should gain:

- `cursor?: number | null`
- `pageSize?: number`

New Browse state should include something like:

- `items`
- `nextCursor`
- `hasMore`
- `isLoadingFirstPage`
- `isLoadingNextPage`
- `isRefreshingList`
- `error`

App-level state should also gain:

- `isCatalogOverlayVisible`
- `catalogOverlayMessage`

### Backend/API

`searchPackages()` should return a paginated result shape rather than a full array.

The browser mock backend should be updated to match the same paginated contract so browser fallback stays compatible.

## What To Read First In The Next Session

If resuming after memory compaction, read in this order:

1. `docs/current-stage.md`
2. `docs/plan-v1.md`
3. `src/lib/store.ts`
4. `src/components/BrowseScreen.svelte`
5. `src-tauri/src/services/catalog_service.rs`
6. `scripts/tauri-runner.mjs`

## Constraints To Keep In Mind

- Do not create `README.md` yet
- Use `apply_patch` for manual file edits
- Avoid reverting unrelated user changes
- The repo is currently dirty because of the desktop-runtime milestone work
- The next implementation step should be the Browse performance milestone, not a return to mock-only browser behavior
