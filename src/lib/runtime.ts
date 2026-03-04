import type { RuntimeKind } from "./types";

declare global {
  interface Window {
    __TAURI__?: {
      core?: {
        invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
      };
    };
  }
}

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && typeof window.__TAURI__?.core?.invoke === "function";
}

export function isBrowserFallbackRuntime(): boolean {
  return !isTauriRuntime();
}

export function getRuntimeKind(): RuntimeKind {
  return isTauriRuntime() ? "tauri" : "browser-mock";
}
