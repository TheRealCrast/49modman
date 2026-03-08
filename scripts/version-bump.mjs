import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import fs from "node:fs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const syncScript = path.join(scriptDir, "version-sync.mjs");

const SEMVER_PATTERN =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;

function parseSemver(version) {
  const match = version.match(SEMVER_PATTERN);
  if (!match) {
    throw new Error(`Invalid semantic version: ${version}`);
  }

  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease: match[4] ?? null
  };
}

function formatSemver({ major, minor, patch, prerelease }) {
  const base = `${major}.${minor}.${patch}`;
  return prerelease ? `${base}-${prerelease}` : base;
}

function bumpVersion(version, kind, prereleaseId) {
  const parsed = parseSemver(version);

  if (kind === "major") {
    return formatSemver({
      major: parsed.major + 1,
      minor: 0,
      patch: 0,
      prerelease: null
    });
  }

  if (kind === "minor") {
    return formatSemver({
      major: parsed.major,
      minor: parsed.minor + 1,
      patch: 0,
      prerelease: null
    });
  }

  if (kind === "patch") {
    return formatSemver({
      major: parsed.major,
      minor: parsed.minor,
      patch: parsed.patch + 1,
      prerelease: null
    });
  }

  if (kind !== "prerelease") {
    throw new Error(`Unsupported bump type: ${kind}`);
  }

  if (parsed.prerelease) {
    const parts = parsed.prerelease.split(".");
    if (parts[0] === prereleaseId) {
      const last = parts.at(-1);
      if (last && /^\d+$/.test(last)) {
        parts[parts.length - 1] = String(Number(last) + 1);
      } else {
        parts.push("0");
      }

      return formatSemver({
        major: parsed.major,
        minor: parsed.minor,
        patch: parsed.patch,
        prerelease: parts.join(".")
      });
    }
  }

  return formatSemver({
    major: parsed.major,
    minor: parsed.minor,
    patch: parsed.patch + 1,
    prerelease: `${prereleaseId}.0`
  });
}

function main() {
  const rawArgs = process.argv.slice(2);
  const dryRunIndex = rawArgs.indexOf("--dry-run");
  const isDryRun = dryRunIndex >= 0;
  if (isDryRun) {
    rawArgs.splice(dryRunIndex, 1);
  }

  const kind = rawArgs[0];
  const prereleaseId = rawArgs[1] ?? "rc";

  if (!kind || !["major", "minor", "patch", "prerelease"].includes(kind)) {
    console.error("Usage: node scripts/version-bump.mjs <major|minor|patch|prerelease> [prerelease-id]");
    process.exit(1);
  }

  const packageJsonPath = path.resolve(scriptDir, "..", "package.json");
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
  const nextVersion = bumpVersion(packageJson.version, kind, prereleaseId);

  if (isDryRun) {
    console.log(`[version-bump] Dry run: ${packageJson.version} -> ${nextVersion}`);
    return;
  }

  const result = spawnSync(process.execPath, [syncScript, nextVersion], {
    stdio: "inherit"
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }

  console.log(`[version-bump] ${packageJson.version} -> ${nextVersion}`);
}

try {
  main();
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`[version-bump] ${message}`);
  process.exit(1);
}
