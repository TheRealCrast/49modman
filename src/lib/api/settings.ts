import { invokeCommand } from "./client";
import type { WarningPrefsDto } from "../types";

export function getWarningPrefs(): Promise<WarningPrefsDto> {
  return invokeCommand("get_warning_prefs");
}

export function setWarningPreference(
  kind: "red" | "broken",
  enabled: boolean
): Promise<WarningPrefsDto> {
  return invokeCommand("set_warning_preference", {
    kind,
    enabled
  });
}
