# Overview Missing-Dependency Warning Inconsistency

Last updated: 2026-03-05

## Problem

The Overview installed-mod list shows inconsistent missing-dependency warning badges.

Observed behavior:

1. Open Overview with one mod installed but a required dependency missing.
   - Actual: no warning badge.
   - Expected: warning badge visible.
2. Start installing the missing dependency and open Overview while install is in progress.
   - Actual: warning badge appears.
   - Expected: warning should not persist once dependency is being installed/completes.
3. After install completes, switching away and back to Overview removes warning.
   - This final state is correct.
4. Uninstall the dependency again.
   - Actual: warning badge still missing.
   - Expected: warning badge visible.
5. Install another mod in the same profile that also has missing dependencies.
   - Actual: first mod may show warning, second mod may not.
   - Expected: both mods should show warning.

## Impact

- Users cannot trust Overview warning badges for dependency health.
- Missing dependencies can be hidden, leading to broken profile states without clear UI feedback.

## Likely Causes

- Per-mod dependency checks run in parallel and failures are silently treated as zero warnings.
- Current counting relies on dependency summary aggregation, which can be lossy for exact per-mod requirements.
- Unresolved dependency entries are not consistently counted as missing.

## Expected Fix Direction

- Use exact dependency tree traversal for each installed mod.
- Count unresolved dependency entries as missing.
- Make dependency checks deterministic and resilient (no silent random zeroing on fetch failure).
- Ensure warning refresh reacts to install-state transitions and profile mod-list changes.
- Keep tooltip text explicit: `Missing dependencies ...`.
