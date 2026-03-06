import { invokeCommand } from "./client";
import type {
  ActivateProfileInput,
  ActivationApplyResult,
  BuildRuntimeStageInput,
  LaunchRuntimeStatus,
  MemoryDiagnosticsSnapshot,
  LaunchProfileInput,
  LaunchResult,
  LaunchVanillaInput,
  ProtonRuntime,
  RuntimeStageBuildResult,
  SteamScanResult,
  TrimResourceMemoryResult,
  ValidateV49InstallInput,
  V49ValidationResult,
  VanillaCleanupResult
} from "../types";

export function scanSteamInstallations(): Promise<SteamScanResult> {
  return invokeCommand("scan_steam_installations");
}

export function validateV49Install(
  input: ValidateV49InstallInput = {}
): Promise<V49ValidationResult> {
  return invokeCommand("validate_v49_install", { input });
}

export function buildRuntimeStage(
  input: BuildRuntimeStageInput = {}
): Promise<RuntimeStageBuildResult> {
  return invokeCommand("build_runtime_stage", { input });
}

export function activateProfile(
  input: ActivateProfileInput = {}
): Promise<ActivationApplyResult> {
  return invokeCommand("activate_profile", { input });
}

export function deactivateToVanilla(): Promise<VanillaCleanupResult> {
  return invokeCommand("deactivate_to_vanilla");
}

export function repairActivation(): Promise<VanillaCleanupResult> {
  return invokeCommand("repair_activation");
}

export function getLaunchRuntimeStatus(): Promise<LaunchRuntimeStatus> {
  return invokeCommand("get_launch_runtime_status");
}

export function getMemoryDiagnostics(): Promise<MemoryDiagnosticsSnapshot> {
  return invokeCommand("get_memory_diagnostics");
}

export function trimResourceSaverMemory(): Promise<TrimResourceMemoryResult> {
  return invokeCommand("trim_resource_saver_memory");
}

export function launchProfile(input: LaunchProfileInput): Promise<LaunchResult> {
  return invokeCommand("launch_profile", { input });
}

export function launchVanilla(input: LaunchVanillaInput): Promise<LaunchResult> {
  return invokeCommand("launch_vanilla", { input });
}

export function listProtonRuntimes(): Promise<ProtonRuntime[]> {
  return invokeCommand("list_proton_runtimes");
}

export function setPreferredProtonRuntime(runtimeId: string): Promise<void> {
  return invokeCommand("set_preferred_proton_runtime", {
    input: { runtimeId }
  });
}
