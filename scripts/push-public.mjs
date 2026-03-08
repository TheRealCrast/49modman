import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";

const remoteName = "public";
const branchName = "main";
const args = new Set(process.argv.slice(2));
const isDryRun = args.has("--dry-run");
const allowDirty = args.has("--allow-dirty");

function run(command, commandArgs, options = {}) {
  const cwd = options.cwd ?? process.cwd();
  const encoding = options.encoding ?? "utf8";

  return execFileSync(command, commandArgs, {
    cwd,
    encoding,
    stdio: options.stdio ?? ["ignore", "pipe", "pipe"]
  });
}

function git(args, options = {}) {
  return run("git", args, options);
}

function gitInherit(args, options = {}) {
  return run("git", args, {
    ...options,
    stdio: "inherit"
  });
}

function getOutput(args, options = {}) {
  return git(args, options).trim();
}

function ensureCleanWorktree(repoRoot) {
  const status = getOutput(["status", "--porcelain"], { cwd: repoRoot });
  if (status && !allowDirty) {
    throw new Error(
      "Working tree is not clean. Commit or stash changes first, or rerun with --allow-dirty."
    );
  }
}

function main() {
  const repoRoot = getOutput(["rev-parse", "--show-toplevel"]);
  const sourceBranch = getOutput(["rev-parse", "--abbrev-ref", "HEAD"], { cwd: repoRoot });
  const sourceSha = getOutput(["rev-parse", "--short", "HEAD"], { cwd: repoRoot });

  getOutput(["remote", "get-url", remoteName], { cwd: repoRoot });
  ensureCleanWorktree(repoRoot);

  gitInherit(["fetch", "--prune", remoteName], { cwd: repoRoot });

  const remoteRef = `refs/remotes/${remoteName}/${branchName}`;
  let hasRemoteBranch = true;
  try {
    getOutput(["show-ref", "--verify", remoteRef], { cwd: repoRoot });
  } catch {
    hasRemoteBranch = false;
  }

  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "modman49-public-sync-"));
  const worktreePath = path.join(tempRoot, "worktree");
  let worktreeAdded = false;

  try {
    if (hasRemoteBranch) {
      gitInherit(["worktree", "add", "--detach", worktreePath, remoteRef], { cwd: repoRoot });
    } else {
      gitInherit(["worktree", "add", "--detach", worktreePath, "HEAD"], { cwd: repoRoot });
      gitInherit(["checkout", "--orphan", branchName], { cwd: worktreePath });
    }
    worktreeAdded = true;

    gitInherit(["rm", "-r", "--ignore-unmatch", "."], { cwd: worktreePath });
    gitInherit(["clean", "-fdx"], { cwd: worktreePath });

    const archiveBuffer = run(
      "git",
      ["archive", "--format=tar", "HEAD", "--", ".", ":(exclude)docs/**"],
      { cwd: repoRoot, encoding: "buffer" }
    );

    const archivePath = path.join(tempRoot, "public-sync.tar");
    fs.writeFileSync(archivePath, archiveBuffer);
    run("tar", ["-xf", archivePath, "-C", worktreePath], { encoding: "utf8" });
    fs.rmSync(archivePath, { force: true });

    if (fs.existsSync(path.join(worktreePath, "docs"))) {
      throw new Error("Filtered snapshot unexpectedly contains docs/. Aborting push.");
    }

    gitInherit(["add", "-A"], { cwd: worktreePath });
    const stagedStatus = getOutput(["status", "--porcelain"], { cwd: worktreePath });
    if (!stagedStatus) {
      console.log("[public:push] No changes to publish.");
      return;
    }

    const commitMessage = `chore(public): sync ${sourceBranch}@${sourceSha} without docs`;
    gitInherit(["commit", "-m", commitMessage], { cwd: worktreePath });

    if (isDryRun) {
      console.log("[public:push] Dry run complete. Commit created locally but not pushed:");
      gitInherit(["--no-pager", "log", "-1", "--oneline"], { cwd: worktreePath });
      return;
    }

    gitInherit(["push", remoteName, `HEAD:${branchName}`], { cwd: worktreePath });
    console.log(`[public:push] Synced ${sourceBranch}@${sourceSha} to ${remoteName}/${branchName}.`);
  } finally {
    if (worktreeAdded) {
      try {
        gitInherit(["worktree", "remove", "--force", worktreePath], { cwd: repoRoot });
      } catch (error) {
        console.error(`[public:push] Warning: failed to remove temporary worktree: ${error}`);
      }
    }

    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
}

try {
  main();
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`[public:push] ${message}`);
  process.exit(1);
}
