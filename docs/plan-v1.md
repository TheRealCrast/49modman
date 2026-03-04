# 49modman v1 Plan

## Summary

Build `49modman` as a desktop app focused only on **Lethal Company v49** in v1, using **Tauri 2 + Rust + Svelte 5 + TypeScript**. The app should be lightweight, fast, stable, and easy to distribute to:

- Windows 10/11 users
- Arch Linux users, including i3 setups

The first release will provide:

- Thunderstore browsing for Lethal Company
- per-version browsing and installation
- exact-version dependency-aware installs
- date-based v49 compatibility zoning
- higher-priority local-reference statuses for **verified** and **broken** versions
- fast ZIP caching
- per-profile modpacks
- local mod import
- share-file export/import via `.49pack`
- modded and vanilla launch flows
- Windows direct/Steam launch
- Linux direct launch with Proton selection and Steam launch support
- a guided manual workflow for acquiring and validating the depot-swapped v49 install
- easy in-app editing of locally stored verified/broken version references

The first release will not include:

- automated Steam depot download
- a hosted backend
- hosted short-code sharing
- multi-game support
- `README.md` before the first build milestone

## Product Scope

### In Scope for v1

- Lethal Company only
- v49 only
- standard Steam installs only
- guided v49 setup and validation
- Thunderstore catalog sync
- package browser with compatibility coloring and flairs
- package detail page with version history
- red-version and broken-version warnings with override
- exact version installation
- ZIP caching
- profiles as modpacks
- local mod import
- `.49pack` import/export
- vanilla/modded launch switching
- direct and Steam launch options
- Linux Proton selection for direct launch only
- local editing of version reference states

### Out of Scope for v1

- automatic depot acquisition
- backend services
- hosted share codes
- package-manager distribution
- multi-version engine
- full generic config editor UI
- cloud sync
- README before first successful build milestone

## Compatibility and Reference Model

Compatibility is version-specific and determined in this priority order:

1. `broken`
2. `verified`
3. `green`
4. `yellow`
5. `orange`
6. `red`

### Status Definitions

- `broken`: locally marked as confirmed non-working/problematic for v49
- `verified`: locally marked as confirmed working for v49
- `orange`: uploaded **before December 9, 2023**
- `green`: uploaded **on/after December 9, 2023** and **before March 31, 2024**
- `yellow`: uploaded **on/after March 31, 2024** and **before April 13, 2024**
- `red`: uploaded **on/after April 13, 2024**

### Visual Rules

- `broken` uses a distinct **bright red** flair/state
- `verified` uses a distinct **bright green** flair/state
- `broken` and `verified` are local-reference overrides for an exact mod version
- `broken` overrides `verified` if conflicting data exists after merge
- `verified` overrides the date-based zone for that exact version
- `red` remains the date-window incompatible class for post-April-13-2024 versions

### Browser Defaults

Default visible statuses:

- `verified`
- `green`
- `yellow`
- `orange`

Hidden by default:

- `red`
- `broken`

Rationale: `broken` should be visible when specifically browsing version history or if the user enables it, but should not dominate the main discovery experience by default.

### Package Card Classification

- If any version is `verified`, the card uses bright green and recommends the newest verified version
- If no verified version exists but a version is marked `broken`, do not promote the package card to broken by default unless every v49-relevant version is broken
- Otherwise classify by the newest v49-relevant eligible version using:
  - `green` over `yellow` over `orange`
- If newer incompatible versions exist, show a badge such as `Newer incompatible versions exist`
- If the recommended version is locally marked `broken`, show a bright red broken flair on the card and require deliberate install confirmation

### Versions Tab Behavior

Per-version list shows:

- version number
- publish date
- base date zone
- effective status
- install action
- warning state if red or broken
- reference source for verified/broken state
- filters/toggles for:
  - verified
  - broken
  - green
  - yellow
  - orange
  - red

## Local Reference Data

### Source Model

Use both:

- a bundled reference dataset shipped with the app
- an optional user-editable override dataset

### Reference States

Each exact package version may have one local reference state:

- `verified`
- `broken`
- unset

### Identity Rules

Key each reference entry by:

- `package_full_name`
- `version_number`

Optionally persist stronger identifiers when available:

- package UUID
- version UUID
- checksum

### Merge Rules

- bundled dataset is read-only
- override dataset is editable in-app
- override dataset wins over bundled dataset
- if an override clears a bundled state, store that as an explicit neutral override
- if conflicting override data somehow exists, `broken` wins over `verified`

### Intended File Locations

- bundled: `src-tauri/resources/v49-reference.json`
- override: `$app_data/state/v49-reference.override.json`

## Architecture

### Stack

- Tauri 2
- Rust backend with `tokio`
- Svelte 5 frontend with TypeScript
- SQLite via `sqlx`
- `reqwest` for HTTP
- `tracing` for logging

### Initial Repo Structure

```text
49modman/
  docs/
    plan-v1.md
    architecture.md
    manifest-format.md
    launch-flow.md
  src/
    app/
    routes/
    components/
    stores/
    lib/
    types/
  src-tauri/
    resources/
      v49-reference.json
    src/
      main.rs
      commands/
      catalog/
      downloads/
      profiles/
      launcher/
      steam/
      proton/
      import_export/
      reference/
      validation/
      db/
      logging/
```

## Data Model

### SQLite Tables

#### `packages`

- `uuid`
- `namespace`
- `name`
- `full_name`
- `owner`
- `community`
- `categories_json`
- `icon_url`
- `website_url`
- `is_deprecated`
- `total_downloads`
- `rating_score`
- `last_synced_at`

#### `package_versions`

- `uuid`
- `package_uuid`
- `version_number`
- `published_at`
- `base_zone`
- `effective_status`
- `download_url`
- `file_size`
- `dependencies_json`
- `is_active`
- `downloads`
- `description`
- `sha256`
- `manifest_json`

`base_zone` is one of `orange|green|yellow|red`.

`effective_status` is one of `broken|verified|orange|green|yellow|red`.

#### `version_references`

- `id`
- `package_full_name`
- `version_number`
- `version_uuid`
- `state` (`verified`, `broken`, `neutral`)
- `source` (`bundled`, `override`)
- `notes`
- `updated_at`

#### `profiles`

- `id`
- `name`
- `slug`
- `game_install_path`
- `created_at`
- `updated_at`
- `last_played_at`
- `launch_mode_default`
- `notes`

#### `profile_mods`

- `profile_id`
- `source_type`
- `package_uuid`
- `version_uuid`
- `local_mod_id`
- `enabled`
- `pinned`
- `installed_at`

#### `local_mods`

- `id`
- `profile_id`
- `name`
- `version`
- `source_path`
- `import_kind`
- `normalized_dir`
- `checksum`

#### `app_settings`

Store:

- Steam paths
- Proton runtime selection
- browser filters
- cache limit
- default launch mode
- `warn_red_downloads` boolean, default `true`
- `warn_broken_downloads` boolean, default `true`

## Public Interfaces and Types

### Tauri Commands

```ts
scanSteamInstallations(): Promise<SteamScanResult>
validateV49Install(gamePath: string): Promise<V49ValidationResult>
syncCatalog(force?: boolean): Promise<CatalogSyncResult>
searchPackages(query: PackageSearchQuery): Promise<Paginated<PackageCard>>
getPackageDetail(packageUuid: string): Promise<PackageDetail>
getVersionStatuses(packageUuid: string): Promise<PackageVersionRow[]>
listVersionReferences(query?: VersionReferenceQuery): Promise<VersionReferenceRow[]>
setVersionReference(input: SetVersionReferenceInput): Promise<VersionReferenceRow>
clearVersionReference(input: ClearVersionReferenceInput): Promise<void>
importReferenceDataset(path: string): Promise<ReferenceImportResult>
exportReferenceDataset(path: string): Promise<ReferenceExportResult>
createProfile(input: CreateProfileInput): Promise<ProfileSummary>
cloneProfile(profileId: string, newName: string): Promise<ProfileSummary>
installPackageVersion(input: InstallVersionInput): Promise<InstallResult>
toggleProfileMod(input: ToggleProfileModInput): Promise<void>
importLocalMod(input: ImportLocalModInput): Promise<LocalModSummary>
exportProfile(input: ExportProfileInput): Promise<ExportResult>
importProfileFile(path: string): Promise<ImportPreview>
activateProfile(profileId: string): Promise<ActivationResult>
launchProfile(input: LaunchProfileInput): Promise<LaunchResult>
launchVanilla(input: LaunchVanillaInput): Promise<LaunchResult>
listProtonRuntimes(): Promise<ProtonRuntime[]>
setPreferredProtonRuntime(runtimeId: string): Promise<void>
setWarnRedDownloads(enabled: boolean): Promise<void>
setWarnBrokenDownloads(enabled: boolean): Promise<void>
getDiagnosticsBundlePath(): Promise<string>
```

### Key Types

```ts
type BaseZone = "orange" | "green" | "yellow" | "red";
type ReferenceState = "verified" | "broken" | "neutral";
type EffectiveStatus = "broken" | "verified" | BaseZone;

type PackageCard = {
  packageUuid: string;
  fullName: string;
  recommendedVersion: string | null;
  latestOverallVersion: string;
  effectiveStatus: EffectiveStatus;
  newerIncompatibleExists: boolean;
  categories: string[];
  downloads: number;
  ratingScore: number;
};

type PackageVersionRow = {
  versionUuid: string;
  versionNumber: string;
  publishedAt: string;
  baseZone: BaseZone;
  effectiveStatus: EffectiveStatus;
  referenceState?: "verified" | "broken";
  referenceSource?: "bundled" | "override";
};

type SetVersionReferenceInput = {
  packageFullName: string;
  versionNumber: string;
  state: "verified" | "broken";
  notes?: string;
};

type InstallVersionInput = {
  profileId: string;
  packageUuid: string;
  versionUuid: string;
  bypassRedWarning?: boolean;
  bypassBrokenWarning?: boolean;
};
```

## Thunderstore Integration

### Source of Truth

Use the Thunderstore package API for the Lethal Company community and normalize it locally.

### Sync Behavior

- load cached catalog immediately on startup
- background refresh if cache older than 15 minutes
- manual refresh forces full sync
- normalize all package versions
- compute `base_zone`
- overlay local-reference status from bundled and override datasets
- build a local search index

### Install Behavior

- install exact version ZIP
- cache ZIP by version UUID and checksum
- resolve dependencies from selected version metadata
- fail with clear action if a required dependency version is unavailable

## Warning and Override Flows

### Red Version Warning

When the user attempts to download/install a `red` version:

- show a blocking modal warning
- explain that versions uploaded on or after **April 13, 2024** are treated as incompatible with v49
- allow:
  - cancel
  - continue anyway
- include checkbox:
  - `Do not show this again`

Behavior:

- checkbox updates `app_settings.warn_red_downloads = false`
- settings page must allow re-enabling later
- install still remains possible

### Broken Version Warning

When the user attempts to download/install a `broken` version:

- show a blocking modal warning
- explain that this exact version is locally marked as confirmed broken/problematic for v49
- if notes exist in the local reference, show them in the modal
- allow:
  - cancel
  - continue anyway
- include checkbox:
  - `Do not show this again`

Behavior:

- checkbox updates `app_settings.warn_broken_downloads = false`
- settings page must allow re-enabling later
- install still remains possible

### Warning Scope

Warning suppression is global app behavior, not profile-specific.

## Reference Editing UX

Provide easy access to locally edit version reference states.

### Entry Points

- Versions tab for each package:
  - context actions for `Mark verified`
  - `Mark broken`
  - `Clear local override`
- Settings page:
  - `Version Reference Library` management screen
- Optional quick action from installed mods list:
  - open current version in reference editor

### Reference Library Screen

This screen should support:

- search by mod/package/version
- filter by state:
  - verified
  - broken
  - override only
  - bundled only
- edit notes
- promote bundled entry with local override
- clear local override
- import override dataset from file
- export override dataset to file

### Editing Rules

- users may edit only the override dataset in-app
- bundled entries appear read-only unless overridden
- clearing an override restores the bundled state, if present
- edits should update effective statuses immediately in the UI without full catalog resync

## Game Install and v49 Validation

### Onboarding Flow

1. detect Steam libraries
2. locate Lethal Company install
3. validate whether it matches supported v49 expectations
4. if not valid, show guided manual setup:
   - open Steam console with `steam://open/console`
   - run `download_depot 1966720 1966721 7525563530173177311`
   - copy depot contents into the game install folder
5. re-run validation

### Validation Checks

- executable exists
- required Unity data folder exists
- expected v49 file signature or known hash matches
- folder is writable for activation
- filesystem supports required operations for activation; hardlink preferred, copy fallback always valid

## Launch Design

The launcher is an activate-run-cleanup-capable system with deterministic ownership of mod-loader files.

### Modded Launch Flow

1. validate game install
2. validate profile dependency state
3. build or verify staged runtime
4. remove stale app-owned files from previous activation if needed
5. activate selected profile into game install
6. write activation manifest
7. launch game
8. capture logs and exit status

### Vanilla Launch Flow

1. remove active app-owned files using activation manifest
2. verify cleanup
3. launch unmodded game

### Windows Modes

Support all four:

- modded direct launch
- modded Steam launch
- vanilla direct launch
- vanilla Steam launch

#### Windows Modded Direct Launch

- activate profile
- launch real executable directly with required BepInEx/Doorstop environment

#### Windows Modded Steam Launch

- activate profile
- invoke Steam with `-applaunch 1966720`

### Linux Modes

Support all four:

- modded direct launch
- modded Steam launch
- vanilla direct launch
- vanilla Steam launch

#### Linux Direct Launch

- activate profile
- use user-selected Proton runtime
- launch game executable through Proton
- block launch if no valid Proton runtime is configured

#### Linux Steam Launch

- activate profile
- invoke Steam with `-applaunch 1966720`
- Steam’s compatibility tool configuration remains authoritative for this mode

### Launch Mode Rules

- Proton selection is shown only on Linux
- Steam-mode launch on Linux depends on Steam already being configured for compatibility as needed
- switching between vanilla and modded must always go through activation/deactivation logic
- profile switching must clean previous app-owned files first

### Failure Handling

Block or recover on:

- invalid or non-v49 game install
- missing dependency versions
- missing/corrupt cached ZIPs
- failed activation
- missing Proton runtime for Linux direct launch

Recovery actions:

- retry
- repair activation
- return to vanilla
- open diagnostics bundle

## Profiles, Imports, and Sharing

### Profiles Are Modpacks

Each profile owns:

- selected exact mod versions
- enabled/disabled states
- config files
- imported local mods
- staged runtime tree

### Local Mod Import

Supported import types:

- Thunderstore ZIP
- raw DLL
- folder with BepInEx-compatible contents

Rules:

- normalize into profile-owned local mod directory
- show as local mods in Installed view
- include in `.49pack` export
- no dependency inference in v1

### Share-File Format

Use `.49pack` as ZIP archive containing:

```text
manifest.json
profile.json
mods.lock.json
config/BepInEx/config/*
config/BepInEx/plugins/*   # embedded local mods only
notes.txt                  # optional
```

Rules:

- Thunderstore mods are referenced by exact package/version identity
- local imported mods are embedded
- config files are included
- cache files are excluded
- game files are excluded

## UI/UX Plan

### Main Screens

#### Onboarding

- Steam detection
- v49 validation
- guided depot instructions
- install health

#### Profiles

- create/clone/delete profile
- active badge
- launch buttons
- import/export `.49pack`

#### Browse

- search
- status filters
- category filters
- sort options
- bright-green verified state
- bright-red broken flair when relevant

#### Package Detail

- overview tab
- versions tab
- dependency tree
- exact-version install
- status filters/toggles
- reference editing actions

#### Installed

- enabled/disabled mods
- dependency warnings
- local mods
- open config/profile folders

#### Downloads

- queue
- speed
- retry/cancel
- cache hit/miss

#### Settings

- Steam detection
- Proton selector on Linux only
- cache size limit
- diagnostics bundle
- warning preferences
- version reference library editor

### UX Defaults

- visible by default: verified, green, yellow, orange
- hidden by default: red, broken
- package cards show recommended version for v49
- explicit labels distinguish:
  - verified via local reference
  - broken via local reference
  - heuristic-only compatibility

## Implementation Phases

### Phase 1: Foundation

- scaffold Tauri/Svelte app
- add SQLite, paths, logging
- Steam detection
- v49 validation
- onboarding shell
- profile CRUD

### Phase 2: Catalog and Status Model

- Thunderstore sync
- package/version normalization
- date-zone computation
- bundled reference dataset loading
- override dataset loading
- effective status overlay
- browse screen
- package detail and versions tab

### Phase 3: Install, Cache, and Profiles

- ZIP cache
- dependency resolver
- exact-version installs
- local mod import
- profile staging

### Phase 4: Launch System

- activation/deactivation manifest
- Windows direct/Steam launch
- Linux direct launch with Proton selection
- Linux Steam launch
- vanilla/modded switching
- repair activation

### Phase 5: Reference Editing and Safety UX

- version reference library screen
- inline mark verified/broken actions
- import/export override reference dataset
- red-version warning modal
- broken-version warning modal
- warning preference toggles

### Phase 6: Sharing and Packaging

- `.49pack` import/export
- diagnostics bundle
- Windows installer
- Linux AppImage

### Phase 7: First Build Milestone

- internal smoke test
- release artifacts
- only after this milestone, add `README.md`

## Test Cases and Scenarios

### Unit Tests

- date-boundary zone classification
- verified overrides base zone
- broken overrides verified and base zone
- package card picks newest verified version when present
- package card falls back correctly when only broken versions are overridden
- red-warning preference persistence
- broken-warning preference persistence
- reference merge rules for bundled plus override data
- clearing local override restores bundled state
- dependency resolution for exact versions
- cache keying and reuse
- activation manifest generation
- hardlink fallback to copy
- `.49pack` roundtrip

### Integration Tests

- catalog sync from fixtures
- bundled reference overlay applies correctly
- override dataset supersedes bundled reference state
- broken override hides bundled verified state
- installing a red version shows warning when enabled
- installing a broken version shows warning when enabled
- disabling future warnings suppresses subsequent popups
- install exact version with dependencies into clean profile
- switch active profiles without stale files
- activate modded then restore vanilla
- import local mod and re-export/import `.49pack`
- edit reference state in UI and see package/version status update immediately

### End-to-End Tests

#### Windows 10/11

- valid v49 install accepted
- invalid build rejected
- verified version appears bright green
- broken version appears bright red
- red version can still be installed after warning confirmation
- broken version can still be installed after warning confirmation
- modded direct launch works
- modded Steam launch works
- vanilla after modded works

#### Arch Linux + i3

- Steam library detection
- v49 validation on ext4/xfs
- official Proton and GE-Proton detection
- direct launch with chosen Proton works
- Steam launch works when compatibility is configured in Steam
- vanilla cleanup works after modded play

## Rollout and Diagnostics

- local file logs only
- diagnostics bundle contains:
  - app logs
  - launch logs
  - activation manifest
  - profile manifest
  - Steam/Proton detection info
- no telemetry
- release targets:
  - Windows installer
  - Linux AppImage

## Important Interface Changes

Compared with the previous plan, these are the important additions:

- add `broken` as a first-class local-reference status
- store local references in a unified bundled-plus-override reference model
- add in-app editing for verified/broken states
- add reference import/export APIs
- add separate warning preferences for red and broken installs
- keep installs permissive: users can still continue anyway
- keep `README.md` deferred until after first build

## Assumptions and Defaults

- v1 supports one known-good v49 build signature
- users own the game on Steam
- depot acquisition remains manual but guided
- share files replace hosted share codes in v1
- verified and broken data are local only
- bundled reference data plus override editing/import is sufficient for v1
- red-version warning is enabled by default
- broken-version warning is enabled by default
- users may bypass either warning and install anyway
- Steam-mode launch on Linux uses Steam’s compatibility tool; direct mode uses the app-selected Proton runtime
- symlinks are avoided as the primary install strategy
- `README.md` must not be created before the first build milestone
