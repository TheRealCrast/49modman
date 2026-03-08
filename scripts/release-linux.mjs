import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const tauriRunnerScript = path.join(scriptDir, "tauri-runner.mjs");

const runtimeUrl =
  "https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64";

function resolveRuntimePath() {
  const customPath = process.env.LDAI_RUNTIME_FILE;
  if (customPath && customPath.trim().length > 0) {
    return customPath;
  }

  const cacheHome =
    process.env.XDG_CACHE_HOME && process.env.XDG_CACHE_HOME.trim().length > 0
      ? process.env.XDG_CACHE_HOME
      : path.join(os.homedir(), ".cache");

  return path.join(cacheHome, "tauri", "runtime-x86_64");
}

async function downloadRuntime(runtimePath) {
  const parent = path.dirname(runtimePath);
  fs.mkdirSync(parent, { recursive: true });

  const response = await fetch(runtimeUrl);
  if (!response.ok) {
    throw new Error(`Failed to download runtime (status ${response.status})`);
  }

  const tempPath = `${runtimePath}.tmp`;
  const bytes = Buffer.from(await response.arrayBuffer());
  fs.writeFileSync(tempPath, bytes);
  fs.chmodSync(tempPath, 0o755);
  fs.renameSync(tempPath, runtimePath);
}

async function ensureRuntime(runtimePath) {
  if (fs.existsSync(runtimePath)) {
    fs.chmodSync(runtimePath, 0o755);
    return;
  }

  console.log(`[release:linux] Downloading AppImage runtime to ${runtimePath}`);
  try {
    await downloadRuntime(runtimePath);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(
      `${message}. Download it manually from ${runtimeUrl} and place it at ${runtimePath}.`
    );
  }
}

function runBuild(runtimePath) {
  const env = {
    ...process.env,
    NO_STRIP: "1",
    LDAI_RUNTIME_FILE: runtimePath
  };

  const result = spawnSync(process.execPath, [tauriRunnerScript, "build", "--bundles", "appimage"], {
    stdio: "inherit",
    env
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

async function main() {
  const runtimePath = resolveRuntimePath();
  await ensureRuntime(runtimePath);
  runBuild(runtimePath);
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`[release:linux] ${message}`);
  process.exit(1);
});
