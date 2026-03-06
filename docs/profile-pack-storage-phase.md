# Profile Pack Storage Reduction Phase

## Summary

Reduce `.49pack` share-file size significantly by moving to a compact-first export model, while still allowing selected embedded fallback payloads for mod versions that are no longer downloadable from Thunderstore metadata.

This phase keeps import compatibility with existing full-payload packs and adds background post-import hydration for missing non-embedded mods.

## Problem Statement

Current `.49pack` export includes:

- metadata docs
- all installed `mods/*` payload directories
- `runtime/BepInEx/config/*`
- `runtime/BepInEx/plugins/*`

This creates very large share files and defeats the purpose of a lightweight profile-sharing format.

## Locked Product Decisions

- New export behavior is compact-first.
- Export preflight must auto-detect unavailable mod versions from local catalog metadata.
- If unavailable versions exist, show a Yes/No/Cancel confirmation modal:
  - `Yes`: embed only unavailable mod payloads.
  - `No`: export compact-only (no embedded payloads).
  - `Cancel`: abort export.
- If no unavailable versions exist, export proceeds with no modal.
- New compact exports include:
  - `manifest.json`
  - `profile.json`
  - `mods.lock.json`
  - `config/BepInEx/config/*` only
- New compact exports do not include `config/BepInEx/plugins/*`.
- Import remains backward compatible with older full-payload packs.
- Import hydration is background and non-blocking.
- Hydration failures are partial and non-fatal:
  - continue remaining items
  - show warning summary for failures/skips
- Hydration install queue must target the imported profile explicitly (not whichever profile is active later).

## Current Baseline Constraints

- Export currently writes every installed mod payload directory into the archive.
- Import currently extracts `mods/*`, `config/BepInEx/config/*`, and `config/BepInEx/plugins/*`.
- Imported manifest entries without extracted install dirs are pruned by manifest reconciliation.
- Download/install queue currently targets active profile unless changed.

## Format Contract (New + Legacy)

## Legacy pack (still supported)

May contain:

- full `mods/*` payloads
- `config/BepInEx/plugins/*`
- `config/BepInEx/config/*`

## New compact/hybrid pack

Must contain:

- `manifest.json`
- `profile.json`
- `mods.lock.json`
- optional `config/BepInEx/config/*`

May contain:

- selective `mods/*` payload directories for unavailable versions only

Must not contain by default:

- `config/BepInEx/plugins/*`
- full payload of all installed mods

## Unavailable-Version Detection Rule

A mod entry in `mods.lock.json` is considered unavailable for compact export preflight when either:

1. exact `package_id + version_id` row is missing from `package_versions` join path, or
2. exact version row exists but `download_url` is empty/whitespace.

This detection is local-catalog based only (no network probe in export flow).

## Milestone Sequence And Exit Gates

### P0: Export Preflight + Modal Wiring

Implement:

- backend preflight command for export readiness:
  - returns installed mods
  - marks unavailable candidates
- frontend export action updated to call preflight first
- new export decision modal in Profiles flow for unavailable-mod case

Exit gate:

- exporting a profile with at least one unavailable version always shows Yes/No/Cancel modal with exact affected list.
- exporting a profile with zero unavailable versions bypasses modal and proceeds directly.

### P1: Compact/Hybrid Export Writer

Implement:

- export input supports `embedUnavailablePayloads` boolean
- export pack writer rules:
  - always write metadata docs
  - always write runtime config subtree (`config/BepInEx/config/*`) when present
  - never write runtime plugins subtree in new exports
  - write `mods/*` payloads only when `embedUnavailablePayloads = true`, and only for unavailable list
- keep ZIP deflate settings unchanged

Exit gate:

- same profile produces materially smaller archive in compact mode than current full-payload baseline.
- selecting `Yes` embeds only unavailable payload subset, not all installed mods.

### P2: Import Compatibility + Pack Detection

Implement:

- importer accepts both:
  - old full-payload packs
  - new compact/hybrid packs
- extraction behavior:
  - extract `mods/*` if present
  - extract `config/BepInEx/config/*` if present
  - extract `config/BepInEx/plugins/*` if present (legacy compatibility only)
- no hard dependency on pack schema version for compatibility path

Exit gate:

- old `.49pack` roundtrip remains functional.
- compact pack import succeeds even when archive has zero `mods/*` files.

### P3: Post-Import Background Hydration

Implement:

- derive hydration set as:
  - versions listed in preview/import metadata
  - minus versions already installed after import extraction/reconciliation
- queue hydration installs in background
- continue on individual queue/install failures
- collect and report failures in import activity feedback

Required API change:

- extend cache queue input to accept optional `profileId`.
- queue/install backend must use explicit `profileId` when provided.

Exit gate:

- switching active profile during hydration does not redirect queued installs to the wrong profile.
- partial failure still leaves imported profile usable and reports failures clearly.

### P4: UX Copy, Diagnostics, and Hardening

Implement:

- activity/toast copy distinguishes:
  - imported immediately from pack payload
  - queued for hydration
  - failed/skipped
- maintain existing warning preference behavior for import preview modal
- add targeted logs for:
  - unavailable detection
  - compact vs hybrid export path
  - hydration queue outcomes

Exit gate:

- users can understand final import state without inspecting logs.

## API And Type Changes

Backend:

- add `preview_export_profile_pack(profile_id)` command and DTOs
- change `export_profile_pack` command to receive structured input:
  - `profile_id`
  - `embed_unavailable_payloads`
- extend `queue_install_to_cache` input:
  - optional `profile_id`

Frontend:

- add preflight export API wrapper + DTO types
- add unavailable-mod export modal state + component
- update export action flow in store:
  - preflight
  - modal resolution
  - export invoke with selected mode
- update hydration queue callsites to pass imported profile id

Browser mock:

- add no-op/mock-safe versions of new command shapes so browser runtime remains type-consistent.

## Data Flow

Export:

1. user clicks `Export .49pack`
2. frontend calls export preflight
3. if unavailable list empty: export compact directly
4. if unavailable list non-empty: modal Yes/No/Cancel
5. frontend invokes export with selected `embedUnavailablePayloads` mode

Import:

1. user selects `.49pack`, preview shown
2. user confirms import
3. importer creates profile, extracts whatever payload exists, writes manifest
4. frontend computes missing mods for hydration
5. frontend queues hydration installs targeting imported profile id
6. final activity summarizes imported + queued + failed counts

## Test Plan

### Unit/service tests

- export preflight marks unavailable versions correctly:
  - missing version row
  - empty `download_url`
  - normal downloadable row
- export writer includes/excludes paths per mode:
  - compact (`No`)
  - hybrid unavailable-only (`Yes`)
- queue install uses explicit `profile_id` when provided

### Integration tests (manual + automated where feasible)

- import old full-payload pack:
  - installed mods restored immediately
  - no unnecessary hydration queue
- import compact pack:
  - zero immediate mod payload restored
  - hydration queue starts for lockfile entries
- import hybrid pack:
  - embedded unavailable subset restored immediately
  - remaining mods hydrated in background
- switch active profile during hydration:
  - queued installs still target imported profile

### Regression checks

- existing profile export/import buttons and warning modal behavior still work
- Downloads list reflects hydration jobs like normal install jobs
- uninstall/enable/disable flows remain unchanged for successfully hydrated mods

## Out Of Scope For This Phase

- local mod import system redesign
- remote hosted share manifests/codes
- catalog auto-refresh policy changes specifically for export
- checksum-based validation of embedded fallback payload correctness beyond existing extraction/install behavior

## Completion Criteria

This phase is complete when:

1. compact export is default path and produces much smaller files.
2. unavailable-version fallback embedding is user-controlled via Yes/No/Cancel modal.
3. old full-payload `.49pack` files remain importable.
4. post-import hydration runs in background against imported profile id.
5. partial hydration failures are surfaced without aborting overall import.

## Implementation Status (2026-03-06)

All milestones P0-P4 are implemented in the current codebase.

Delivered behavior:

- Export preflight is wired and returns unavailable version entries.
- Export decision modal supports `Yes`/`No`/`Cancel`:
  - `Yes`: hybrid export (embed only unavailable payloads).
  - `No`: compact export (no mod payload embedding).
  - `Cancel`: abort.
- New exports include metadata + `config/BepInEx/config/*`; runtime plugins payload is not included in new exports.
- Import preview/import detect pack payload mode (`compact`/`hybrid`/`full`) and legacy runtime plugin payload presence.
- Post-import hydration runs in background and queues against the imported profile id (not active profile).
- Import activity copy now explicitly reports:
  - imported immediately from pack payload
  - queued for hydration
  - failed/skipped
- Targeted diagnostics were added for:
  - unavailable detection totals/reasons
  - export path mode (`compact` vs `hybrid`)
  - hydration queue outcomes/failures

Hardening fixes applied after rollout:

- Backend preview DTOs now always serialize empty arrays for `unavailable_mods` and `mods` to avoid `undefined` array reads in UI.
- Frontend export/import modal/store code now defensively normalizes missing arrays to `[]`.
- Hydration queue version resolution now falls back from `(package_id, version_id)` lookup to `version_id` lookup when package ids drift across catalog snapshots, with a diagnostic log when fallback remaps package id.

Notes:

- Linux warning text such as `"applications.menu" not found in QList("/home/.../.config/menus")` originates from desktop dialog integration and is non-blocking for profile pack logic.
