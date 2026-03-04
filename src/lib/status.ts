import type { EffectiveStatus, ModPackage, ModVersion, ReferenceState } from "./types";

const statusPriority: Record<EffectiveStatus, number> = {
  verified: 5,
  green: 4,
  yellow: 3,
  orange: 2,
  broken: 1,
  red: 0
};

function parseVersionIdentifierPart(part: string) {
  return /^\d+$/.test(part) ? Number.parseInt(part, 10) : part.toLowerCase();
}

function normalizeVersion(value: string) {
  const trimmed = value.trim().replace(/^v/i, "");
  const [withoutBuild] = trimmed.split("+", 1);
  const [corePart, prereleasePart] = withoutBuild.split("-", 2);
  const core = corePart.split(".").map((part) => Number.parseInt(part, 10));

  if (core.length === 0 || core.some((part) => Number.isNaN(part))) {
    return null;
  }

  while (core.length < 3) {
    core.push(0);
  }

  return {
    core,
    prerelease: prereleasePart ? prereleasePart.split(".").map(parseVersionIdentifierPart) : []
  };
}

function comparePrereleaseParts(left: Array<number | string>, right: Array<number | string>) {
  const length = Math.max(left.length, right.length);

  for (let index = 0; index < length; index += 1) {
    const leftPart = left[index];
    const rightPart = right[index];

    if (leftPart === undefined) {
      return 1;
    }

    if (rightPart === undefined) {
      return -1;
    }

    if (typeof leftPart === "number" && typeof rightPart === "number") {
      if (leftPart !== rightPart) {
        return leftPart > rightPart ? 1 : -1;
      }
      continue;
    }

    if (typeof leftPart === "number") {
      return -1;
    }

    if (typeof rightPart === "number") {
      return 1;
    }

    const comparison = leftPart.localeCompare(rightPart);
    if (comparison !== 0) {
      return comparison > 0 ? 1 : -1;
    }
  }

  return 0;
}

export function compareVersionNumbers(left: string, right: string) {
  const normalizedLeft = normalizeVersion(left);
  const normalizedRight = normalizeVersion(right);

  if (normalizedLeft && normalizedRight) {
    const length = Math.max(normalizedLeft.core.length, normalizedRight.core.length);

    for (let index = 0; index < length; index += 1) {
      const leftPart = normalizedLeft.core[index] ?? 0;
      const rightPart = normalizedRight.core[index] ?? 0;

      if (leftPart !== rightPart) {
        return leftPart > rightPart ? 1 : -1;
      }
    }

    const leftHasPrerelease = normalizedLeft.prerelease.length > 0;
    const rightHasPrerelease = normalizedRight.prerelease.length > 0;

    if (leftHasPrerelease !== rightHasPrerelease) {
      return leftHasPrerelease ? -1 : 1;
    }

    const prereleaseComparison = comparePrereleaseParts(
      normalizedLeft.prerelease,
      normalizedRight.prerelease
    );

    if (prereleaseComparison !== 0) {
      return prereleaseComparison;
    }
  }

  return left.localeCompare(right, undefined, { numeric: true, sensitivity: "base" });
}

function compareRecommendedCandidates(left: ModVersion, right: ModVersion) {
  const statusScore = statusPriority[resolveEffectiveStatus(left)] - statusPriority[resolveEffectiveStatus(right)];

  if (statusScore !== 0) {
    return statusScore;
  }

  const versionScore = compareVersionNumbers(left.versionNumber, right.versionNumber);
  if (versionScore !== 0) {
    return versionScore;
  }

  const publishedScore = left.publishedAt.localeCompare(right.publishedAt);
  if (publishedScore !== 0) {
    return publishedScore;
  }

  return left.downloads - right.downloads;
}

export function currentReferenceState(version: ModVersion): Exclude<ReferenceState, "neutral"> | undefined {
  if (version.overrideReferenceState === "broken") {
    return "broken";
  }

  if (version.overrideReferenceState === "verified") {
    return "verified";
  }

  if (version.overrideReferenceState === "neutral") {
    return undefined;
  }

  return version.bundledReferenceState;
}

export function currentReferenceSource(version: ModVersion): "bundled" | "override" | undefined {
  if (version.overrideReferenceState !== undefined) {
    return "override";
  }

  if (version.bundledReferenceState) {
    return "bundled";
  }

  return undefined;
}

export function currentReferenceNote(version: ModVersion): string | undefined {
  if (version.overrideReferenceState !== undefined) {
    return version.overrideReferenceNote || undefined;
  }

  return version.bundledReferenceNote;
}

export function resolveEffectiveStatus(version: ModVersion): EffectiveStatus {
  if (version.effectiveStatus) {
    return version.effectiveStatus;
  }

  const reference = currentReferenceState(version);

  if (reference === "broken") {
    return "broken";
  }

  if (reference === "verified") {
    return "verified";
  }

  return version.baseZone;
}

export function pickRecommendedVersion(pkg: ModPackage): ModVersion {
  return [...pkg.versions].sort((left, right) => compareRecommendedCandidates(right, left))[0];
}

export function everyRelevantVersionBroken(pkg: ModPackage): boolean {
  const relevant = pkg.versions.filter((version) => version.baseZone !== "red");

  return relevant.length > 0 && relevant.every((version) => resolveEffectiveStatus(version) === "broken");
}
