import { invokeCommand } from "./client";
import { isTauriRuntime } from "../runtime";

export async function openExternalUrl(url: string): Promise<void> {
  if (isTauriRuntime()) {
    await invokeCommand("open_external_url", { url });
    return;
  }

  window.open(url, "_blank", "noopener,noreferrer");
}
