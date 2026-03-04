import { invokeCommand } from "./client";
import type { DownloadJobDto, InstallTaskDto } from "../types";

export function listActiveDownloads(): Promise<DownloadJobDto[]> {
  return invokeCommand("list_active_downloads");
}

export function getTask(taskId: string): Promise<InstallTaskDto | null> {
  return invokeCommand("get_task", { taskId });
}
