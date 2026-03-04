import { invokeCommand } from "./client";
import type { GetVersionDependenciesInput, VersionDependenciesDto } from "../types";

export function getVersionDependencies(
  input: GetVersionDependenciesInput
): Promise<VersionDependenciesDto> {
  return invokeCommand("get_version_dependencies", { input });
}

export function warmDependencyIndex(): Promise<void> {
  return invokeCommand("warm_dependency_index");
}
