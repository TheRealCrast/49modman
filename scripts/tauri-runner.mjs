import { spawn } from "node:child_process";
import { homedir } from "node:os";
import { join } from "node:path";

const mode = process.argv[2];

if (!mode || !["dev", "build"].includes(mode)) {
  console.error("Usage: node scripts/tauri-runner.mjs <dev|build> [args...]");
  process.exit(1);
}

const cargoBin = join(homedir(), ".cargo", "bin");
const extraArgs = process.argv.slice(3);
const env = {
  ...process.env,
  PATH: `${cargoBin}:${process.env.PATH ?? ""}`
};

if (process.platform === "linux") {
  env.WEBKIT_DISABLE_DMABUF_RENDERER = "1";
  env.WEBKIT_DISABLE_COMPOSITING_MODE = "1";
  env.GDK_BACKEND = env.GDK_BACKEND ?? "x11";
  env.WINIT_UNIX_BACKEND = env.WINIT_UNIX_BACKEND ?? "x11";
}

const child = spawn("tauri", [mode, ...extraArgs], {
  stdio: "inherit",
  env,
  shell: false
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 1);
});
