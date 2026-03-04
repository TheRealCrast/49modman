import { invokeCommand } from "./client";
import type { CacheSummaryDto, QueueInstallToCacheInput, QueueInstallToCacheResult } from "../types";

export function queueInstallToCache(
  input: QueueInstallToCacheInput
): Promise<QueueInstallToCacheResult> {
  return invokeCommand("queue_install_to_cache", { input });
}

export function getCacheSummary(): Promise<CacheSummaryDto> {
  return invokeCommand("get_cache_summary");
}

export function openCacheFolder(): Promise<void> {
  return invokeCommand("open_cache_folder");
}

export function clearCache(): Promise<CacheSummaryDto> {
  return invokeCommand("clear_cache");
}
