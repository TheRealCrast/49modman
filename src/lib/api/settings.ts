import { invokeCommand } from "./client";
import type {
  StartStorageMigrationInput,
  StorageLocationsDto,
  StorageMigrationStatusDto,
  WarningPrefsDto
} from "../types";

export function getWarningPrefs(): Promise<WarningPrefsDto> {
  return invokeCommand("get_warning_prefs");
}

export function setWarningPreference(
  kind:
    | "red"
    | "broken"
    | "installWithoutDependencies"
    | "uninstallWithDependants"
    | "importProfilePack"
    | "conserveWhileGameRunning",
  enabled: boolean
): Promise<WarningPrefsDto> {
  return invokeCommand("set_warning_preference", {
    input: {
      kind,
      enabled
    }
  });
}

export function getStorageLocations(): Promise<StorageLocationsDto> {
  return invokeCommand("get_storage_locations");
}

export function getStorageMigrationStatus(): Promise<StorageMigrationStatusDto> {
  return invokeCommand("get_storage_migration_status");
}

export function pickStorageFolder(kind: "cache" | "profiles"): Promise<string | null> {
  return invokeCommand("pick_storage_folder", {
    input: {
      kind
    }
  });
}

export function startStorageMigration(
  input: StartStorageMigrationInput
): Promise<StorageMigrationStatusDto> {
  return invokeCommand("start_storage_migration", { input });
}
