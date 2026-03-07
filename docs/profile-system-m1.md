# Profile System Milestone 1

## Summary

Implement the first backend-backed profile milestone for `49modman`, with no install, download, cache, or modpack behavior yet.

This milestone replaces the current frontend-mock profile system with a real SQLite-backed profile system and wires the Profiles tab to it. The app will always ship with a built-in `Default` profile so the UI can never start in a zero-profile state.

Locked product decisions:

- Profiles are real backend state now.
- The app always has a built-in `Default` profile.
- `Default` is undeletable.
- If the active non-default profile is deleted, the app switches back to `Default`.
- Overview keeps the installed-mods section, but for this milestone it reflects real empty state.
- Profile creation supports full fields:
  - name
  - notes
  - game path
  - default launch mode
- Profile creation/editing UI should be an inline app UI in the Profiles view.
- In Settings, the current warning toggles live under a `Warn options` subcategory.
- In Settings, add a `Danger zone` subcategory with a `Reset all data` row.
- `Reset all data` must require confirmation and then wipe all user data back to a fresh first-launch state.

## Scope

### In scope

- SQLite schema for real profiles
- active-profile persistence
- auto-seeded built-in `Default` profile
- backend profile CRUD limited to:
  - list
  - create
  - delete
  - set active
  - get active/detail
- Tauri commands and frontend API for profiles
- Profiles tab UI:
  - show real profiles
  - create real profiles with full fields
  - delete real profiles
  - switch active profile
- Overview wired to real active profile data
- Settings layout update:
  - `Warn options` subcategory
  - `Danger zone` subcategory
  - `Reset all data` action with confirmation
- full user-data reset behavior
- remove mock profile ownership from the Svelte store

### Out of scope

- installs
- downloads
- cache
- modpacks
- rename/duplicate
- launch
- game-folder activation
- import/export
- any persistence of installed mods

## Current baseline

On the reverted baseline:

- there is no backend profile command surface in `src-tauri/src/main.rs`
- the store still owns profiles as mock frontend state in `src/lib/store.ts`
- the Profiles tab is UI-only in `src/components/ProfilesScreen.svelte`
- the `Profile` type is a frontend mock type in `src/lib/types.ts`
- the Overview screen currently reads profile-installed mods from the mock profile object in `src/components/OverviewScreen.svelte`
- Settings currently exposes the warning toggles directly, without subcategories or a danger-zone section

## Database schema

Add a new migration:

- `src-tauri/migrations/0003_profiles.sql`

### `profiles` table

```sql
CREATE TABLE IF NOT EXISTS profiles (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  notes TEXT NOT NULL,
  game_path TEXT NOT NULL,
  launch_mode_default TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  last_played_at TEXT NULL,
  is_builtin_default INTEGER NOT NULL DEFAULT 0
);
```

Rules:

- exactly one profile may be flagged as built-in default in normal seeded state
- `is_builtin_default = 1` identifies the undeletable `Default` profile
- do not rely on the name `Default` alone for protection

### settings additions

Persist the active profile in the existing `settings` table with key:

- `profiles.active_id`

Add a reset-safe seeded default for it in `seed_defaults()`.

## Default profile seeding

Extend `src-tauri/src/db/mod.rs` so `seed_defaults()` also guarantees:

1. `profiles.active_id` exists in `settings`
2. a built-in `Default` profile exists
3. if no active profile is set, the built-in `Default` profile becomes active

### Seeded `Default` profile values

- `id`: `default`
- `name`: `Default`
- `notes`: `Built-in fallback profile.`
- `game_path`: `""`
- `launch_mode_default`: `"steam"`
- `is_builtin_default`: `1`
- `last_played_at`: `NULL`

## Backend services and commands

Add:

- `src-tauri/src/services/profile_service.rs`
- `src-tauri/src/commands/profiles.rs`

Register these commands:

- `list_profiles`
- `get_active_profile`
- `set_active_profile`
- `create_profile`
- `delete_profile`
- `get_profile_detail`
- `reset_all_data`

### DTOs

#### `ProfileSummaryDto`

```ts
type ProfileSummaryDto = {
  id: string;
  name: string;
  notes: string;
  gamePath: string;
  lastPlayed: string | null;
  launchModeDefault: "steam" | "direct";
  installedCount: number;
  enabledCount: number;
  isBuiltinDefault: boolean;
};
```

For this milestone:

- `installedCount = 0`
- `enabledCount = 0`

#### `ProfileDetailDto`

```ts
type ProfileDetailDto = {
  id: string;
  name: string;
  notes: string;
  gamePath: string;
  lastPlayed: string | null;
  launchModeDefault: "steam" | "direct";
  isBuiltinDefault: boolean;
  installedMods: [];
};
```

#### `CreateProfileInput`

```ts
type CreateProfileInput = {
  name: string;
  notes?: string;
  gamePath?: string;
  launchModeDefault?: "steam" | "direct";
};
```

### Reset command

Add:

```ts
resetAllData(): Promise<void>
```

Behavior:

1. delete all rows from:
   - `profiles`
   - `settings`
   - any future user-owned tables if present
2. preserve schema only
3. rerun the same default-seeding logic used on first app launch
4. result must be equivalent to a clean first launch:
   - built-in `Default` exists
   - warning settings reset to defaults
   - `profiles.active_id = "default"`
   - no other user profiles remain

For this milestone, because installs/downloads/cache are out of scope, resetting user data only needs to cover SQLite-backed user state. File-system deletion is not required yet.

## Profile service rules

### List ordering

Use:

1. built-in `Default` first
2. then other profiles by `updated_at DESC`
3. tie-break by `name ASC`

### Create rules

- trim `name`
- reject empty trimmed names
- reject names that exactly duplicate an existing profile name, case-insensitive
- default missing fields:
  - `notes = ""`
  - `gamePath = ""`
  - `launchModeDefault = "steam"`
- new profile becomes active immediately

### Delete rules

- if `profile_id` is the built-in default profile:
  - reject with `DEFAULT_PROFILE_PROTECTED`
- if deleted profile was active:
  - switch active profile to `default`
- otherwise keep current active profile unchanged

### Active profile repair

If `profiles.active_id` points to a missing profile:

- reset it to `default`

## Frontend API

Add:

- `src/lib/api/profiles.ts`

Functions:

- `listProfiles()`
- `getActiveProfile()`
- `setActiveProfile(profileId: string)`
- `createProfile(input: CreateProfileInput)`
- `deleteProfile(profileId: string)`
- `getProfileDetail(profileId: string)`
- `resetAllData()`

## Frontend state changes

Update `src/lib/types.ts` and `src/lib/store.ts`.

### Replace mock profile model

Replace current mock profile ownership with backend DTOs:

- `ProfileSummaryDto`
- `ProfileDetailDto`
- `CreateProfileInput`

### `AppState` additions

```ts
profiles: ProfileSummaryDto[]
activeProfile?: ProfileDetailDto
isLoadingProfiles: boolean
profileError: string | null
isResettingData: boolean
settingsError: string | null
```

`downloads` can remain mock or empty for now, but must no longer be driven by profile creation/deletion.

## Store behavior

### Bootstrap

During `actions.bootstrap()`:

1. load warning prefs and catalog summary as today
2. load real profiles from backend
3. set:
   - `profiles`
   - `activeProfile`
   - `selectedProfileId`
4. continue existing catalog/bootstrap behavior

### Profile actions

Implement:

- `loadProfilesState()`
- `refreshActiveProfile()`
- `selectProfile(profileId)`
- `createProfile(input)`
- `deleteSelectedProfile()`
- `resetAllData()`

### Reset behavior

When the user confirms reset:

1. set `isResettingData = true`
2. call backend `resetAllData()`
3. clear frontend profile-related state
4. reload warning prefs, profiles, and active profile from backend
5. leave the app in the same state as a fresh first launch

Do not silently preserve old frontend state after reset.

## UI changes

### Profiles tab

Update `src/components/ProfilesScreen.svelte`.

Required features:

- render real profile summaries
- mark active profile
- allow switching active profile
- allow creating a profile via inline form
- allow deleting the selected profile
- disable delete for the built-in default profile

#### Create UI

Use an inline form inside the Profiles panel with fields:

- Name
- Notes
- Game path
- Default launch mode

Actions:

- `Create`
- `Cancel`

### Overview

Update `src/components/OverviewScreen.svelte`.

- read real `activeProfile`
- keep installed mods section visible
- installed list is empty for now
- empty copy: `No mods installed yet.`

### Settings

Update `src/components/SettingsScreen.svelte`.

#### `Warn options` subcategory

Move the existing warning toggles under a subcategory heading:

- `Warn options`

The existing toggles remain unchanged functionally.

#### `Danger zone` subcategory

Add a second subcategory:

- `Danger zone`

Under it, add one row:

- `Reset all data`

Row behavior:

- clicking it opens a confirmation popup
- confirmation copy should clearly say all user profile/settings data will be erased and rebuilt to a fresh default state
- on confirm, call `resetAllData()`

The row should use a danger/destructive visual treatment, but not dominate the page.

## Failure handling

### Profile errors

Use `profileError` for:

- blank name
- duplicate name
- missing profile
- protected default deletion

### Reset errors

Use `settingsError` or a dedicated reset error surfaced in Settings if reset fails.

### Reset confirmation

The reset action must never run without explicit confirmation.

## Test cases

### Backend

- fresh DB creates built-in `Default`
- fresh DB sets `profiles.active_id = "default"`
- missing active id is repaired to `default`
- missing default profile is recreated by seed logic
- create with all fields succeeds
- create with blank name fails
- create with duplicate name fails case-insensitively
- new profile becomes active
- deleting non-default profile succeeds
- deleting active non-default profile switches active back to `default`
- deleting `default` fails
- reset all data removes non-default profiles and restores `Default`
- reset all data restores warning prefs defaults

### Frontend/manual smoke

- first app launch shows real `Default` profile
- Profiles tab renders with `Default`
- creating a profile from inline form works
- new profile becomes active immediately
- topbar selector and Profiles list stay in sync
- deleting a non-default profile works
- deleting the active non-default profile switches back to `Default`
- delete button is disabled for `Default`
- Overview shows the active profile and an empty installed-mods section
- Settings shows:
  - `Warn options`
  - `Danger zone`
  - `Reset all data`
- reset confirmation appears before reset runs
- after reset, app is back to a fresh first-launch state
- app restart preserves created profiles and active selection

## Acceptance criteria

1. Profiles are backend-backed SQLite data, not frontend mock state.
2. The app always starts with a built-in `Default` profile.
3. `Default` cannot be deleted.
4. Profiles tab supports real create, delete, and active switching.
5. New profile creation supports full fields.
6. Deleting the active non-default profile falls back to `Default`.
7. Overview uses real active profile data and shows empty installed-mod state.
8. Settings warning toggles live under `Warn options`.
9. Settings includes a `Danger zone` section with `Reset all data`.
10. `Reset all data` restores a fresh clean-slate user state after confirmation.
11. `npm run build` passes.
12. `cargo check --manifest-path src-tauri/Cargo.toml` passes.

## Assumptions and defaults

- The built-in default profile uses fixed ID `default`.
- The built-in default profile name is exactly `Default`.
- Profile names must be unique case-insensitively.
- Newly created profiles become active immediately.
- Installed mods remain an empty array in this milestone.
- Rename and duplicate are deferred.
- Delete confirmation uses an in-app modal.
- Reset confirmation uses an explicit in-app modal.
- Game path is metadata only for now.
- Default launch mode options are only `steam` and `direct`.
- The Profiles tab owns the inline create form UI.
- Reset-all-data only needs to reset SQLite-backed user state in this milestone.
