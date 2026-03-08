# Onboarding O5 Regression Matrix

Last updated: 2026-03-08

This matrix is the manual acceptance sweep for onboarding O5.

## Manual Scenarios

| ID | Scenario | Steps | Expected |
|---|---|---|---|
| O5-M1 | First run hard-gate | Start with fresh app data, launch app. | App opens to onboarding, nav/topbar hidden, non-onboarding views blocked. |
| O5-M2 | Auto-detect path fill | On onboarding required screen, wait for detection. | Detection loads, `onboardingPathDraft` fills from detected `selectedGamePath` when available. |
| O5-M3 | Invalid path guidance | Set invalid folder path and validate. | Validation fails, checks list renders, targeted guidance text appears. |
| O5-M4 | Depot workflow guidance | Force mismatch/unconfigured signature and validate. | Depot guidance card renders with Steam console action + command copy action. |
| O5-M5 | Required-mode completion handoff | In required mode, validate a good install. | Completion is persisted, onboarding unlocks app immediately, full bootstrap continues to normal shell. |
| O5-M6 | Restart bypass | After O5-M5, restart app. | App does not re-gate; onboarding is skipped automatically. |
| O5-M7 | Settings manual rerun entry | Go to Settings -> Onboarding -> Run. | Onboarding opens in manual mode, nav/topbar hidden while onboarding view is active. |
| O5-M8 | Manual mode exit | In manual onboarding, use `Back to Settings`. | Returns to Settings without toggling hard gate. |
| O5-M9 | Manual mode completion | In manual onboarding, validate a good install. | Completion metadata is refreshed and flow returns to Settings. |
| O5-M10 | Active profile path sync | Active profile has blank (or same-as-draft) game path, then complete onboarding. | Active profile game path updates to validated path; no profile data regression. |

## Automated Coverage Reference

- Backend:
  - `db::tests::seed_defaults_includes_onboarding_keys`
  - `settings_service::tests::onboarding_status_defaults_to_not_completed`
  - `settings_service::tests::complete_onboarding_persists_status`
  - `settings_service::tests::complete_onboarding_rejects_empty_game_path`
  - `settings_service::tests::onboarding_status_tolerates_malformed_values`
- Frontend/store:
  - `store onboarding bootstrap branches` (Vitest):
    - enters required onboarding when incomplete
    - skips re-gate path when completed
    - blocks leaving onboarding only in required mode
