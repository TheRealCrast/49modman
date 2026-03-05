# Profile Manifest Schema v1

This documents the `manifest.json` shape used at:

- `$APP_DATA/profiles/<profile_id>/manifest.json`

## Top-level object

```json
{
  "schemaVersion": 1,
  "updatedAt": "2026-03-04T23:57:56.409214341Z",
  "profile": {
    "id": "default",
    "name": "Default",
    "notes": "Built-in fallback profile.",
    "gamePath": "",
    "launchModeDefault": "steam",
    "isBuiltinDefault": true
  },
  "mods": [
    {
      "packageId": "bepinex-pack",
      "packageName": "BepInEx-BepInExPack",
      "versionId": "bepinex-5417",
      "versionNumber": "5.4.2100",
      "enabled": true,
      "sourceKind": "thunderstore",
      "installDir": "mods/BepInEx-BepInExPack-5.4.2100",
      "installedAt": "2026-03-04T23:44:04.003936763Z"
    }
  ]
}
```

## Field definitions

- `schemaVersion`: integer schema version. Current value: `1`.
- `updatedAt`: RFC 3339 timestamp for last manifest rewrite.
- `profile`: embedded profile metadata snapshot.
- `mods`: installed mod entries for this profile.

## `profile` object

- `id`: profile ID.
- `name`: display name.
- `notes`: freeform notes.
- `gamePath`: game path override.
- `launchModeDefault`: `"steam"` or `"direct"`.
- `isBuiltinDefault`: whether this is the built-in default profile.

## `mods[]` entries

- `packageId`: package identifier.
- `packageName`: package full name.
- `versionId`: exact version identifier.
- `versionNumber`: display version.
- `enabled`: active toggle state in Overview.
- `sourceKind`: current value is `"thunderstore"`.
- `installDir`: profile-relative install directory (usually under `mods/`).
- `installedAt`: RFC 3339 timestamp for when this entry was installed/refreshed.

## Reconciliation behavior

- On profile manifest reads, entries whose `installDir` path does not exist are automatically pruned from `mods[]`.
- When pruning occurs, `manifest.json` is rewritten atomically and `updatedAt` is refreshed.
