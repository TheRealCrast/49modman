# Launch System Phase Plan

Last updated: 2026-03-06

This document expands Phase 4 of `docs/plan-v1.md` into an implementation-ready launch plan for `49modman`.

## Summary

Implement a deterministic, cross-platform launch system for Lethal Company v49 that:

- validates game/install prerequisites before launch
- assembles a profile-scoped BepInEx runtime from installed enabled mods
- activates and deactivates app-owned files safely
- supports modded and vanilla launch modes on Windows and Linux
- captures launch diagnostics and exposes recovery actions

This phase starts from current state (`cache + profile install + profile manifest`) and adds the missing activation/launch foundation.

## Current State (As Implemented)

- Profiles are real backend state with per-profile folders in app data.
- Profile manifests persist installed Thunderstore mods and enabled flags.
- Install extracts cached zip archives to `profiles/<id>/mods/<package>-<version>/`.
- Runtime scaffold folders exist at:
  - `profiles/<id>/runtime/BepInEx/plugins/`
  - `profiles/<id>/runtime/BepInEx/config/`
- Topbar launch buttons are wired to backend launch commands for modded and vanilla launch flows.

## Implementation Status (L0-L5)

- `L0` completed: game-path resolution + v49 validation + dependency checks.
- `L1` completed: deterministic runtime stage builder with stale temp cleanup.
- `L2` completed: activation/deactivation engine with ownership manifest.
- `L3` completed: launch execution + diagnostics artifacts.
- `L4` completed: Linux/Proton runtime detection + direct/steam launch handling.
- `L5` completed: UI wiring for launch state, repair action, and diagnostics open.

## Post-Implementation Notes

- Runtime stage normalization now handles common Thunderstore layout variance:
  - strips wrapper roots such as `BepInExPack/`
  - remaps top-level `plugins`, `patchers`, `config`, `core` into `BepInEx/...`
  - remaps `.lethalbundle` payloads from non-anchor roots into `BepInEx/plugins/...` so LethalLevelLoader auto-detects bundle content
- Activation stale cleanup is now non-blocking for retained non-empty managed directories; only remaining managed files block new activation.
- Linux direct launch sets `WINEDLLOVERRIDES=winhttp=n,b` in-process for Doorstop/BepInEx injection reliability.
- Linux Steam launch remains dependent on Steam compatibility environment; when Steam is already running, game-specific launch options may still be required.
- Linux modded Steam launch now performs a preflight check against Steam app launch options and returns a user-friendly remediation message if required options are missing.
- Steam launch-options parsing now supports escaped quotes in `localconfig.vdf`, preventing false negatives for valid values such as `WINEDLLOVERRIDES=\"winhttp=n,b\" %command%`.
- Steam launch-option validation now treats any launch-options value containing both `%command%` and `winhttp=n,b` as valid, so existing extra flags/args do not cause false failures.
- Launch feedback UI was refined:
  - feedback stack no longer expands to fill the primary content row
  - feedback panel text now preserves multiline instructions and wraps safely
  - warning/success heading spacing and icon mapping were corrected
- Topbar launch labels now append mode suffix only for direct mode (`(Direct)`), not Steam mode.
- Proton runtime selection is now grouped under Settings -> `Launch (Linux)`.
- Dependency-state precheck now validates only enabled Thunderstore-installed entries (`sourceKind: "thunderstore"`), so enabled local `.zip` imports are excluded from catalog dependency enforcement.
- Launch now supports an explicit dependency validation bypass (`skip_dependency_validation`) for retry flows:
  - when enabled, precheck records `PROFILE_DEPENDENCY_STATE_SKIPPED` and continues launch
  - this is intended for the specific `PROFILE_DEPENDENCY_STATE_INVALID` recovery path

## Required End State For This Phase

- Launch commands exist and are wired to UI actions.
- Launch preflight validates install, v49 signature, and environment requirements.
- Runtime staging is deterministic from profile state (`manifest.json` + enabled flags).
- Activation writes/removes only app-owned files and records ownership in an activation manifest.
- Modded and vanilla launch flows both work with cleanup guarantees.
- Linux direct launch supports explicit Proton runtime selection.
- Failures return actionable error codes and recovery affordances.

## Scope

### In scope

- launch preflight checks and blockers
- runtime staging builder
- activation/deactivation engine
- Windows direct and Steam launch modes
- Linux direct (Proton) and Steam launch modes
- launch progress/result DTOs and error handling
- diagnostics/log capture for launch paths
- launch UI wiring and status feedback

### Out of scope

- auto depot download/acquisition
- `.49pack` sharing/import/export
- local mod import UX beyond already installed content
- hosted telemetry/services

## Launch Architecture

All launch operations follow a strict pipeline:

1. resolve target profile and launch request
2. run preflight validation
3. stage runtime from active profile
4. cleanup stale activation (if present)
5. activate target profile into game install
6. launch process (direct or Steam)
7. capture status/log pointers
8. expose cleanup/repair path when needed

Vanilla launch flow:

1. load last activation manifest
2. remove app-owned files from game dir
3. verify cleanup
4. start unmodded game process

### Ownership model

- App never deletes files it did not create or mark as managed.
- Activation manifest is the source of truth for cleanup.
- A failed activation must leave enough state for `repair_activation` to restore vanilla.

### Runtime model

- Per-profile staged runtime lives under `profiles/<id>/runtime/`.
- Game-install activation copies or links from stage into game root.
- Hardlink is preferred when supported, copy fallback is always valid.
- Symlinks are not required for v1.

## Data Contracts

### Settings keys

Store launch-related preferences in `settings`:

- `launch.default_mode` -> `"steam"` or `"direct"`
- `launch.preferred_game_path` -> string path
- `launch.preferred_proton_runtime_id` -> string or null (Linux only)
- `launch.last_mode` -> `"modded"` or `"vanilla"` (optional convenience)
- `launch.last_profile_id` -> string or null

### Activation manifest v1

Path:

- `$APP_DATA/state/activation-manifest-v1.json`

Required fields:

- `schemaVersion: 1`
- `createdAt`
- `updatedAt`
- `profileId`
- `gamePath`
- `platform` (`windows` | `linux`)
- `mode` (`modded`)
- `entries[]` where each entry includes:
  - `relativePath`
  - `kind` (`file` | `dir`)
  - `source` (`stage` | `generated`)
  - `operation` (`copy` | `hardlink`)
  - `sha256` (optional for debug/repair)

### Launch log paths

Store launch artifacts under:

- `$APP_DATA/logs/launch/<timestamp>/launch.json`
- `$APP_DATA/logs/launch/<timestamp>/stdout.log` (when available)
- `$APP_DATA/logs/launch/<timestamp>/stderr.log` (when available)

## Command Surface (Phase Deliverables)

Add backend commands:

- `scan_steam_installations() -> SteamScanResult`
- `validate_v49_install(gamePath: string) -> V49ValidationResult`
- `list_proton_runtimes() -> ProtonRuntime[]`
- `set_preferred_proton_runtime(runtimeId: string) -> void`
- `launch_profile(input: LaunchProfileInput) -> LaunchResult`
- `launch_vanilla(input: LaunchVanillaInput) -> LaunchResult`
- `repair_activation(input?: RepairActivationInput) -> RepairActivationResult`
- `get_launch_diagnostics_path() -> string`

### Key types

`LaunchMode`:

- `"direct"`
- `"steam"`

`LaunchProfileInput`:

- `profileId: string`
- `launchMode: LaunchMode`
- `gamePathOverride?: string`
- `protonRuntimeId?: string` (Linux direct only)
- `skipDependencyValidation?: boolean`

`LaunchVanillaInput`:

- `launchMode: LaunchMode`
- `gamePathOverride?: string`
- `protonRuntimeId?: string` (Linux direct only)

`LaunchResult`:

- `ok: boolean`
- `code: string`
- `message: string`
- `pid?: number`
- `usedGamePath?: string`
- `usedProfileId?: string`
- `usedLaunchMode?: LaunchMode`
- `diagnosticsPath?: string`

`PreflightResult`:

- `ok: boolean`
- `code: string`
- `message: string`
- `checks: PreflightCheck[]`

`ProtonRuntime`:

- `id`
- `displayName`
- `path`
- `source` (`steam` | `custom`)
- `isValid`

## BepInEx And Lethal Company Runtime Rules

- BepInEx runtime assembly must honor Thunderstore package installer layout expectations.
- Installed package contents are sourced from profile `mods/` install dirs and filtered by `enabled`.
- Runtime staging target structure is BepInEx-compatible and deterministic.
- Profile-disabled mods are excluded from staging.
- Missing or invalid installed mod directories fail preflight with clear package/version context.
- Linux direct launch sets required Wine/Proton environment for Doorstop/BepInEx injection.
- Steam-mode launches on Linux rely on Steam compatibility settings as authoritative.

## Milestone Sequence And Exit Gates

Reference convention for this phase:

- Prefer BepInEx v5.4.21 docs when possible because Lethal Company mod stacks are typically BepInEx 5-based.
- Use BepInEx master docs only when the v5 page is missing or materially less clear.

### L0: Launch Preflight Foundation

Implement:

- game path resolution order:
  - explicit input override
  - active profile `gamePath`
  - stored preferred game path
  - Steam scan result fallback
- v49 validation checks:
  - executable exists
  - Unity data folder exists
  - supported v49 signature/hash
  - path writable for activation
  - filesystem capability (hardlink support detection; copy fallback allowed)
- dependency-state validation for enabled installed mods

Exit gate:

- `validate_v49_install` returns deterministic pass/fail codes with actionable messages.

Reference docs:

- BepInEx installation (game root placement, first-run config/log generation):
  - https://docs.bepinex.dev/v5.4.21/articles/user_guide/installation/index.html
- BepInEx troubleshooting (entrypoint/runtime startup issues relevant to preflight checks):
  - https://docs.bepinex.dev/v5.4.21/articles/user_guide/troubleshooting.html

Setup-guide quotes:

> "The game root folder is where the game executable is located."

> "Simply run the game executable. This should generate BepInEx configuration file into `BepInEx/config` folder and an initial log file `BepInEx/LogOutput.log`."

### L1: Runtime Staging Builder

Implement:

- stage directory builder under `profiles/<id>/runtime/active-stage/`
- deterministic merge of enabled installed mod payloads
- replacement semantics when re-staging same profile
- cleanup of stale stage artifacts before rebuild

Exit gate:

- same profile + same enabled set produces stable staged output across repeated runs.

Reference docs:

- BepInEx plugin loading expectation (`BepInEx/plugins`):
  - https://docs.bepinex.dev/v5.4.11/articles/dev_guide/plugin_tutorial/index.html
- BepInEx configuration location (`BepInEx/config/BepInEx.cfg` and plugin cfg files):
  - https://docs.bepinex.dev/v5.4.21/articles/user_guide/configuration.html

### L2: Activation/Deactivation Engine

Implement:

- apply stage into game install with ownership tracking
- write `activation-manifest-v1.json` atomically
- cleanup routine for stale app-owned files before re-activation
- vanilla deactivation from manifest with post-check verification

Exit gate:

- `activate -> vanilla cleanup -> activate` cycles leave no unmanaged side effects.

Reference docs:

- BepInEx installation contract (drop-in extraction into game root):
  - https://docs.bepinex.dev/v5.4.21/articles/user_guide/installation/index.html
- BepInEx troubleshooting notes tied to managed/core files and proxy DLL behavior (`winhttp.dll`):
  - https://docs.bepinex.dev/v5.4.21/articles/user_guide/troubleshooting.html

Setup-guide quote:

> "The game root folder is where the game executable is located."

### L3: Launch Execution Core

Implement:

- Windows direct launch process creation
- Windows Steam launch via `-applaunch 1966720`
- launch result capture (`pid`, status, diagnostics path)
- launch-time error mapping (`PRECHECK_FAILED`, `ACTIVATION_FAILED`, `LAUNCH_FAILED`, etc.)

Exit gate:

- both Windows launch modes start successfully from activated state.

Reference docs:

- BepInEx Steam interop workflow (Steam launch-script integration patterns):
  - https://docs.bepinex.dev/master/articles/advanced/steam_interop.html
- BepInEx installation first-run behavior (expected config/log side effects after launch):
  - https://docs.bepinex.dev/v5.4.21/articles/user_guide/installation/index.html

### L4: Linux/Proton Paths

Implement:

- Proton runtime detection and validation
- Linux direct launch through selected Proton runtime
- Linux Steam launch path
- explicit failure when Linux direct launch has no valid Proton runtime

Exit gate:

- Linux direct launch works with selected runtime; Linux Steam launch works when Steam compatibility is configured.

Reference docs:

- BepInEx Proton/Wine guide (`winhttp` override and Proton compatibility behavior):
  - https://docs.bepinex.dev/articles/advanced/proton_wine.html
- BepInEx Steam interop guide for Unix/Steam launch-script expectations:
  - https://docs.bepinex.dev/master/articles/advanced/steam_interop.html

Setup-guide quote:

> "BepInEx relies on `winhttp.dll` proxy DLL to inject itself into Unity games."

### L5: UI Wiring, Repair, Diagnostics

Implement:

- wire topbar `Launch modded` and `Launch vanilla` buttons to new commands
- show launch progress and non-blocking result feedback
- provide repair action when activation or cleanup inconsistency is detected
- expose diagnostics folder open path from launch errors

Exit gate:

- user can complete modded launch, vanilla return, and repair flows entirely from UI.

Reference docs:

- BepInEx logging behavior and disk log outputs (`BepInEx/LogOutput.log` / listeners):
  - https://docs.bepinex.dev/v5.4.11/articles/dev_guide/plugin_tutorial/4_logging.html
- BepInEx troubleshooting guidance for surfacing actionable startup diagnostics:
  - https://docs.bepinex.dev/v5.4.21/articles/user_guide/troubleshooting.html

Setup-guide quote:

> "Simply run the game executable. This should generate BepInEx configuration file into `BepInEx/config` folder and an initial log file `BepInEx/LogOutput.log`."

## Failure Model And Recovery

Block launch on:

- invalid or missing game path
- non-v49 install
- dependency-state mismatch in enabled Thunderstore mods (unless `skip_dependency_validation` is set)
- missing/corrupt required mod files
- activation failure
- missing Proton runtime for Linux direct launch

Recovery actions:

- retry preflight/launch
- rebuild stage and retry activation
- run `repair_activation`
- run `launch_vanilla` after successful repair
- open diagnostics bundle path

## Test Plan

### Unit tests

- preflight path resolution order
- v49 signature check boundaries
- stage builder determinism from enabled mod set
- activation manifest generation and atomic write
- ownership-safe cleanup (never delete unmanaged files)
- launch-mode validation rules per platform

### Integration tests

- install enabled mods -> stage -> activate -> deactivate cycle
- profile switch triggers stale managed cleanup
- launch failure preserves repairable activation state
- Linux direct launch fails fast without valid Proton runtime

### End-to-end smoke tests

Windows:

- modded direct launch
- modded Steam launch
- vanilla cleanup and launch after modded session

Linux:

- modded direct launch with selected Proton
- modded Steam launch with compatibility configured
- vanilla cleanup and launch after modded session

## Implementation Defaults And Assumptions

- Steam app id is `1966720`.
- Launch system is local-only with no telemetry.
- Activation manifest schema is versioned from day one (`schemaVersion = 1`).
- Hardlink is preferred where possible; copy fallback is mandatory.
- Existing profile manifest (`schemaVersion = 1`) remains source of installed mod state.
- Existing warning preferences and install workflows remain unchanged by this phase.
