import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");

const packageJsonPath = path.join(repoRoot, "package.json");
const tauriConfigPath = path.join(repoRoot, "src-tauri", "tauri.conf.json");
const cargoTomlPath = path.join(repoRoot, "src-tauri", "Cargo.toml");
const cargoLockPath = path.join(repoRoot, "src-tauri", "Cargo.lock");

const SEMVER_PATTERN =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function writeJsonIfChanged(filePath, nextValue) {
  const current = fs.readFileSync(filePath, "utf8");
  const next = `${JSON.stringify(nextValue, null, 2)}\n`;
  if (current !== next) {
    fs.writeFileSync(filePath, next);
    return true;
  }

  return false;
}

function writeTextIfChanged(filePath, nextValue) {
  const current = fs.readFileSync(filePath, "utf8");
  if (current !== nextValue) {
    fs.writeFileSync(filePath, nextValue);
    return true;
  }

  return false;
}

function assertSemver(version) {
  if (!SEMVER_PATTERN.test(version)) {
    throw new Error(`Invalid semantic version: ${version}`);
  }
}

function syncCargoTomlVersion(cargoToml, version) {
  const pattern = /(\[package\][\s\S]*?\nversion\s*=\s*")([^"]+)(")/;
  if (!pattern.test(cargoToml)) {
    throw new Error("Failed to locate package version in src-tauri/Cargo.toml");
  }

  return cargoToml.replace(pattern, `$1${version}$3`);
}

function syncCargoLockVersion(cargoLock, version) {
  const pattern = /(\[\[package\]\]\nname\s*=\s*"modman49"\nversion\s*=\s*")([^"]+)(")/;
  if (!pattern.test(cargoLock)) {
    throw new Error("Failed to locate modman49 package version in src-tauri/Cargo.lock");
  }

  return cargoLock.replace(pattern, `$1${version}$3`);
}

function main() {
  const requestedVersion = process.argv[2];
  const packageJson = readJson(packageJsonPath);

  const version = requestedVersion ?? packageJson.version;
  assertSemver(version);

  if (packageJson.version !== version) {
    packageJson.version = version;
  }

  const changed = [];
  if (writeJsonIfChanged(packageJsonPath, packageJson)) {
    changed.push("package.json");
  }

  const tauriConfig = readJson(tauriConfigPath);
  if (tauriConfig.version !== version) {
    tauriConfig.version = version;
  }
  if (writeJsonIfChanged(tauriConfigPath, tauriConfig)) {
    changed.push("src-tauri/tauri.conf.json");
  }

  const cargoToml = fs.readFileSync(cargoTomlPath, "utf8");
  const nextCargoToml = syncCargoTomlVersion(cargoToml, version);
  if (writeTextIfChanged(cargoTomlPath, nextCargoToml)) {
    changed.push("src-tauri/Cargo.toml");
  }

  const cargoLock = fs.readFileSync(cargoLockPath, "utf8");
  const nextCargoLock = syncCargoLockVersion(cargoLock, version);
  if (writeTextIfChanged(cargoLockPath, nextCargoLock)) {
    changed.push("src-tauri/Cargo.lock");
  }

  if (changed.length === 0) {
    console.log(`[version-sync] No changes needed. Version is already ${version}.`);
    return;
  }

  console.log(`[version-sync] Synced version ${version} in:`);
  for (const file of changed) {
    console.log(`- ${file}`);
  }
}

try {
  main();
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`[version-sync] ${message}`);
  process.exit(1);
}
