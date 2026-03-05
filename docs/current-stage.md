# Current Stage Notes

Last updated: 2026-03-04

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
- Reset-all-data UX/backend flow has been hardened:
  - schema-safe reset for legacy tables
  - visible progress modal during reset
  - automatic Browse recache/refresh after reset completes
- Browse card layout has been hardened for narrow windows:
  - list/card containers now enforce shrink bounds (`min-width: 0`)
  - long card text/chips now wrap instead of forcing horizontal overflow
  - prevents package cards from being cut off on the right after startup or Browse refresh

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

Current working tree includes the install-state control follow-up and manifest reconciliation fixes (pending commit).

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
- modpack flows are still not implemented

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
  - source kind (`thunderstore`)
  - install directory under `mods/`
  - installed timestamp
- manifest read APIs now enrich installed-mod DTOs with optional `iconDataUrl` if `icon.png` exists in that mod folder
- manifest reads now also reconcile stale entries by pruning `mods[]` rows whose `installDir` no longer exists on disk
- `reset_all_data` now clears profile folders and then reseeds + re-ensures `default` profile storage
- per-profile storage size is computed from profile directory bytes and returned in `list_profiles`
- Settings profile summary is returned via backend command:
  - `get_profiles_storage_summary`

## Cache + Profile Install Notes

- this is the first real install-adjacent filesystem work since the revert
- install scope is now `cache + active profile extract`
- Downloads behavior is `active only`
- Browse labels stay `Install` / `Install version`, but the detail Install button now shows the exact selected version
- install flow now:
  1. check exact version in shared cache
  2. download to cache on miss
  3. extract cached zip into active profile `mods/<package>-<version>/`
  4. upsert manifest entry for that exact package/version
- repeated installs of the same exact version currently re-extract into the same target folder and refresh manifest timestamp
- multiple versions of the same package are currently allowed side-by-side
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
- install-related download icons now force white fill for consistency across status-colored install buttons:
  - Browse card quick-install button
  - detail `Install {version}` button
  - detail `Install version` buttons
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
