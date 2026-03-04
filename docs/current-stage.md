# Current Stage Notes

Last updated: 2026-03-03

This file is the short-term implementation handoff for the current stage of the project.
The broad product plan remains in [plan-v1.md](./plan-v1.md).

## Current Project State

- Project root: `/home/crast/dev/49modman`
- Git repo is initialized locally on branch `main`
- Current recorded baseline commit: `3826f97` (`Initial frontend and backend scaffold`)
- The app now runs as a real Tauri desktop window instead of only as a browser/Vite page
- The current frontend is still visually close to the browser prototype, but it is now being exercised through the desktop app runtime
- The install/cache/modpack experiment was reverted
- The profile-only milestone is now implemented and working

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
- Overview now uses the real active profile and shows empty installed-mod state
- Settings now includes:
  - `Warn options`
  - `Cache` is not implemented yet
  - `Danger zone`
  - `Reset all data`

Next planned milestone:

- [cache-system-m1.md](./cache-system-m1.md)

Locked next-step focus:

- real global Thunderstore archive cache
- cache-aware `Install` / `Install version`
- active-only Downloads tab backed by real task rows
- Settings support to:
  - open cache folder
  - clear cache
- `Reset all data` upgraded to clear cache files too
- no profile/modpack install state changes yet

## Current Uncommitted Work

At the time these notes were written, the working tree was dirty with:

- `docs/current-stage.md`
- `docs/profile-system-m1.md`
- `src/App.svelte`
- `src/app.css`
- `src/components/OverviewScreen.svelte`
- `src/components/ProfilesScreen.svelte`
- `src/components/SettingsScreen.svelte`
- `src/lib/api/client.ts`
- `src/lib/api/mock-backend.ts`
- `src/lib/api/profiles.ts`
- `src/lib/mock-data.ts`
- `src/lib/store.ts`
- `src/lib/types.ts`
- `src-tauri/migrations/0003_profiles.sql`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/commands/profiles.rs`
- `src-tauri/src/db/mod.rs`
- `src-tauri/src/main.rs`
- `src-tauri/src/services/mod.rs`
- `src-tauri/src/services/profile_service.rs`

## Profile Milestone Notes

- `Default` uses fixed id `default`
- `Default` cannot be deleted
- creating a profile makes it active immediately
- deleting the active non-default profile falls back to `Default`
- `reset_all_data` currently resets SQLite-backed user state only:
  - profiles
  - settings
  - reference overrides
  - cached catalog metadata rows
  - sync state
- install/download/cache/modpack behavior is still intentionally not implemented on this branch
- existing Browse install actions are placeholder-only and do not modify profile state

## Cache Milestone Notes

- this will be the first real install-adjacent filesystem work since the revert
- install scope is `cache only`
- Downloads behavior is `active only`
- Browse labels stay as `Install` / `Install version`
- the current local SQLite DB still contains legacy tables from the reverted experiment:
  - `cached_archives`
  - `install_tasks`
  - `download_jobs`
  - `profile_mods`
  - `profile_mod_dependencies`
  - `local_mods`
- the cache milestone must stay compatible with that DB shape rather than assuming a clean slate

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

Browse performance optimization has now been implemented once, and the current focus has shifted from planning to validation/tuning.

User-reported issue that drove this work:

- significant performance issues in Browse
- likely caused by Thunderstore catalog size
- likely also worsened by rendering too many items at once
- user also wanted a loading overlay while the Thunderstore database is being retrieved

Product decisions already made for this milestone:

- Browse scaling mode: `Infinite scroll`
- Loading overlay scope: `Whole-app overlay`

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
