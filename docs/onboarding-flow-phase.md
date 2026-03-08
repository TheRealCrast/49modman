# Onboarding Flow Phase Plan

Last updated: 2026-03-08

This document expands the onboarding part of `docs/plan-v1.md` into an implementation-ready, multi-phase plan for `49modman`.

## Implementation Status (Current)

As of 2026-03-08, phases O0 through O5 are implemented:

- O0: onboarding settings persistence/DTO/commands are live.
- O1: native game folder picker and onboarding detection wiring are live.
- O2: first-run hard gate and onboarding shell routing are live.
- O3: validation result rendering, guidance mapping, and depot workflow actions are live.
- O4: completion handoff + bootstrap continuation and Settings manual rerun flow are live.
- O5: backend + store regression tests and manual regression matrix are added.

Recent UI polish:

- Added a small top margin above the onboarding `Validate and continue`/`Re-check` primary action row for better spacing.

## Summary

Implement a first-run onboarding flow that:

- hard-gates the app on first launch until game install validation passes
- auto-detects Steam game locations and auto-fills the path field when found
- supports manual game-path entry with native folder picker fallback
- guides users through the manual depot workflow when validation fails
- persists onboarding completion globally (not per-profile)
- does not auto re-gate after completion; users can rerun onboarding from Settings

Locked product decisions:

- hard first-run gate
- global completion scope
- path input includes both text and picker
- show loading transition while detecting game location
- after completion, onboarding is manual rerun only (no automatic re-gate)

## Current State (As Implemented)

Backend already provides most validation primitives:

- `scan_steam_installations() -> SteamScanResult`
- `validate_v49_install(input?: ValidateV49InstallInput) -> V49ValidationResult`
- path resolution precedence in validation:
  - explicit override
  - profile `game_path`
  - `launch.preferred_game_path`
  - Steam scan fallback

Frontend currently has no dedicated onboarding route:

- `AppView` includes only `overview|browse|profiles|downloads|settings`
- app bootstraps directly into normal shell/cached catalog flow
- game path can be edited in Profiles only
- launch preflight blocks invalid installs, but there is no guided first-run setup screen

## Required End State

- First app run shows onboarding before the normal app shell.
- Nav and non-onboarding screens are inaccessible while onboarding is required.
- Onboarding can:
  - detect Steam roots and libraries
  - choose from detected game path(s)
  - browse for folder path
  - validate selected path
  - show check-by-check health output
  - show manual depot instructions when invalid
  - re-run validation
- Successful validation persists onboarding completion metadata.
- Completion transitions to full app bootstrap.
- Settings includes `Run onboarding again` entry point.

## Scope

### In scope

- onboarding status persistence keys
- new onboarding commands/DTOs
- native game-folder picker command
- onboarding UI screen and state machine
- first-run hard-gate behavior in bootstrap/navigation
- manual rerun from Settings
- validation result rendering and manual setup guidance

### Out of scope

- automatic Steam depot download
- depot-content copy automation
- profile-specific onboarding completion
- telemetry/analytics

## Data Contracts

### New settings keys

Persist in existing `settings` table:

- `onboarding.v49.completed` -> `boolean` (default `false`)
- `onboarding.v49.completed_at` -> RFC3339 `string` or absent
- `onboarding.v49.last_validated_game_path` -> `string` or absent

### New DTOs

```ts
type OnboardingStatusDto = {
  completed: boolean;
  completedAt?: string;
  lastValidatedGamePath?: string;
};

type CompleteOnboardingInput = {
  validatedGamePath: string;
};

type PickGameInstallFolderInput = {
  initialPath?: string;
};
```

### Command surface additions

Add commands:

- `get_onboarding_status() -> OnboardingStatusDto`
- `complete_onboarding(input: CompleteOnboardingInput) -> OnboardingStatusDto`
- `pick_game_install_folder(input?: PickGameInstallFolderInput) -> string | null`

Command placement:

- onboarding status commands in `commands/settings.rs` + `services/settings_service.rs`
- game-path picker command in `commands/launch.rs` + `services/launch_service.rs`

Reasoning: onboarding completion is app state, while picker/validation concern launch/game-path workflows.

## Frontend Contract Changes

### Type/store additions

- extend `AppView` with `"onboarding"`
- add onboarding state to `AppState`:
  - `onboardingRequired: boolean`
  - `onboardingMode: "required" | "manual" | null`
  - `onboardingStatus?: OnboardingStatusDto`
  - `onboardingPathDraft: string`
  - `onboardingScan?: SteamScanResult`
  - `onboardingValidation?: V49ValidationResult`
  - loading/error flags:
    - `isLoadingOnboardingStatus`
    - `isDetectingOnboardingPath`
    - `isValidatingOnboardingPath`
    - `onboardingError`

### New UI surface

Add `src/components/OnboardingScreen.svelte` with sections:

- detection state card (spinner + status copy)
- path input row:
  - editable text field
  - `Browse...` button (native folder picker)
  - `Detect again` button
- detected paths list (when scan returns >1 paths)
- validation checks list (`checks[]` with pass/fail badges)
- guided manual setup block for invalid states
- primary actions:
  - `Validate and continue`
  - `Re-check`
  - `Open Steam console`
  - `Copy depot command`

### Shell behavior

- In `App.svelte`, render onboarding view when `onboardingRequired`.
- Hide nav rail and topbar while required onboarding is active.
- Block `actions.setView` from leaving onboarding while required gate is true.
- For manual rerun mode, keep nav hidden only while onboarding screen is active; allow exit back to Settings.

## Boot and Flow Design

## Startup sequence

1. Initialize runtime kind and clear transient state.
2. Load minimal startup set:
   - profiles
   - settings/warning prefs
   - onboarding status
3. If `onboarding.v49.completed === false`:
   - set `view = "onboarding"`
   - set `onboardingRequired = true`
   - run path detection (`scan_steam_installations`)
   - auto-fill draft path from `selectedGamePath` when found
   - stop here (skip catalog bootstrap until onboarding completes)
4. If completed:
   - run existing full bootstrap flow unchanged.

## Validation flow

`Validate and continue`:

1. Build validation input with `gamePathOverride = onboardingPathDraft.trim()` if non-empty.
2. Force `skipDependencyValidation = true` for onboarding.
3. Call `validate_v49_install`.
4. Render result checks and summary.
5. If success:
   - call `complete_onboarding({ validatedGamePath })`
   - optionally update active profile `gamePath` to validated path if blank or same as draft
   - clear onboarding required state
   - continue with full bootstrap
6. If failure:
   - keep user in onboarding
   - show targeted guidance from `code` + `checks`

## Guided manual setup mapping

When validation fails, map these codes to guidance:

- `GAME_PATH_RESOLUTION_FAILED`:
  - ask user to choose folder manually
- `GAME_EXECUTABLE_MISSING` / `GAME_DATA_DIR_MISSING`:
  - likely wrong folder selected
- `V49_SIGNATURE_MISMATCH`:
  - show depot workflow guidance
- `GAME_PATH_NOT_WRITABLE`:
  - show permission/ownership guidance

Manual depot guidance block content:

1. Open Steam console via `steam://open/console`.
2. Run command:
   - `download_depot 1966720 1966721 7525563530173177311`
3. Copy depot files into selected Lethal Company game root.
4. Return and press `Re-check`.

Implementation note: opening `steam://open/console` can reuse existing `open_external_url` command.

## Phase Sequence and Exit Gates

### O0: Onboarding Data Contract Foundation

Implement:

- seed default onboarding setting values
- add onboarding status DTO and service helpers
- add read/write command handlers
- add frontend API bindings and types

Exit gate:

- fresh DB returns `completed=false`
- completing onboarding persists and re-reads stable values

### O1: Native Game Path Picker + Detection Wiring

Implement:

- `pick_game_install_folder` command using native picker
- optional `initialPath` support
- onboarding detection action calling `scan_steam_installations`
- auto-fill `onboardingPathDraft` from detected `selectedGamePath`

Exit gate:

- picker opens and returns path or null
- detect action shows loading transition and updates draft when path is found

### O2: First-Run Hard Gate Shell

Implement:

- add onboarding view/component mount in `App.svelte`
- bootstrap split (minimal first, full after onboarding)
- nav/topbar gating while required onboarding is active
- action guard in `setView`

Exit gate:

- first run cannot access non-onboarding views
- already-complete installs skip onboarding entirely

### O3: Validation + Guided Manual Workflow

Implement:

- validate action + check list rendering
- error-to-guidance mapping by validation `code`
- depot instructions card and steam-console action
- `Re-check` loop

Exit gate:

- failed validations produce actionable onboarding guidance
- success path is unambiguous and repeatable

### O4: Completion Handoff + Manual Rerun

Implement:

- completion write and transition to normal app bootstrap
- optional sync of active profile `gamePath`
- Settings action: `Run onboarding again`
- manual mode behavior (not a persistent gate)

Exit gate:

- onboarding completion unlocks app immediately
- app restart bypasses onboarding
- manual rerun can be entered from Settings without changing global gate semantics

### O5: Regression and Acceptance Sweep

Implement:

- backend tests for status persistence and defaults
- frontend/store tests for bootstrap branch behavior
- manual scenario matrix for first-run and rerun

Exit gate:

- no regressions in existing bootstrap, launch preflight, or profile workflows

## Testing Plan

### Backend tests

- `seed_defaults` includes onboarding defaults
- `get_onboarding_status` handles missing/malformed setting payloads safely
- `complete_onboarding` validates non-empty path and persists timestamp/path

### Frontend/store tests

- bootstrap enters onboarding mode when completion is false
- bootstrap enters normal flow when completion is true
- required onboarding blocks view changes
- onboarding validation success triggers completion and full bootstrap

### Manual scenarios

- first run with detected Steam path
- first run with no detected path (picker flow)
- wrong folder path then correction
- signature mismatch and manual depot guidance
- completed onboarding restart behavior
- Settings rerun behavior when install later becomes invalid

## Risks and Mitigations

- Risk: onboarding blocks too much startup logic.
  - Mitigation: split bootstrap into minimal and full phases; keep full flow unchanged after completion.
- Risk: path auto-fill overwrites user edits.
  - Mitigation: only auto-fill when field is empty or untouched since last detect pass.
- Risk: duplicate source of truth for game path.
  - Mitigation: keep validation source-of-truth in `validate_v49_install`; onboarding only orchestrates.

## Implementation Notes (File Targets)

Primary files expected to change:

- `src/lib/types.ts`
- `src/lib/store.ts`
- `src/App.svelte`
- `src/components/SettingsScreen.svelte`
- `src/components/OnboardingScreen.svelte` (new)
- `src/lib/api/settings.ts`
- `src/lib/api/launch.ts`
- `src/lib/api/client.ts`
- `src/lib/api/mock-backend.ts`
- `src-tauri/src/services/settings_service.rs`
- `src-tauri/src/commands/settings.rs`
- `src-tauri/src/services/launch_service.rs`
- `src-tauri/src/commands/launch.rs`
- `src-tauri/src/db/mod.rs`
- `src-tauri/src/main.rs`

## References

These are supporting docs used to justify implementation choices.

- BepInEx v5 installation guide: https://docs.bepinex.dev/v5.4.21/articles/user_guide/installation/index.html
  - Quote: "The game root folder is where the game executable is located."
  - Quote: "Simply run the game executable. This should generate BepInEx configuration file into `BepInEx/config` folder and an initial log file `BepInEx/LogOutput.log`."
- `rfd` crate docs (native folder picker API): https://docs.rs/rfd/latest/rfd/
  - Quote: "cross platform library for using native file open/save dialogs"
- `webbrowser` crate docs (open external URL, including scheme handlers): https://docs.rs/webbrowser/latest/webbrowser/
  - Quote: "Opens URL on default web browser."
