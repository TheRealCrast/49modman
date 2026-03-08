import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const mode = process.argv[2];
const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "..");

if (!mode || !["dev", "build"].includes(mode)) {
  console.error("Usage: node scripts/tauri-runner.mjs <dev|build> [args...]");
  process.exit(1);
}

const cargoBin = join(homedir(), ".cargo", "bin");
const extraArgs = process.argv.slice(3);
const env = {
  ...process.env,
  PATH: `${cargoBin}${delimiter}${process.env.PATH ?? ""}`
};

if (process.platform === "linux") {
  env.WEBKIT_DISABLE_DMABUF_RENDERER = "1";
  env.WEBKIT_DISABLE_COMPOSITING_MODE = "1";
  env.GDK_BACKEND = env.GDK_BACKEND ?? "x11";
  env.WINIT_UNIX_BACKEND = env.WINIT_UNIX_BACKEND ?? "x11";
}

const localTauriPath = join(
  repoRoot,
  "node_modules",
  ".bin",
  process.platform === "win32" ? "tauri.cmd" : "tauri"
);
const tauriCommand = existsSync(localTauriPath) ? localTauriPath : "tauri";

const child = spawn(tauriCommand, [mode, ...extraArgs], {
  stdio: "inherit",
  env,
  shell: false
});

child.on("error", (error) => {
  if (error.code === "ENOENT") {
    console.error("Failed to find the Tauri CLI.");
    console.error("Run `npm install` to install @tauri-apps/cli, or `cargo install tauri-cli`.");
    process.exit(1);
  }

  console.error(error);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 1);
});
