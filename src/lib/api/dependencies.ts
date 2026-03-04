import { invokeCommand } from "./client";
import type { GetVersionDependenciesInput, VersionDependencyTreeDto } from "../types";

export function getVersionDependencies(
  input: GetVersionDependenciesInput
): Promise<VersionDependencyTreeDto> {
  return invokeCommand("get_version_dependencies", { input });
}
