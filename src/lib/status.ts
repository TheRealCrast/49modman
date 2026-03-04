import type { EffectiveStatus, ModPackage, ModVersion, ReferenceState } from "./types";

const statusPriority: Record<EffectiveStatus, number> = {
  verified: 5,
  green: 4,
  yellow: 3,
  orange: 2,
  red: 1,
  broken: 0
};

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
  const sorted = [...pkg.versions].sort((left, right) => right.publishedAt.localeCompare(left.publishedAt));
  const verified = sorted.find((version) => resolveEffectiveStatus(version) === "verified");

  if (verified) {
    return verified;
  }

  const eligiblePool = sorted.filter((version) =>
    ["green", "yellow", "orange"].includes(resolveEffectiveStatus(version))
  );

  if (eligiblePool.length > 0) {
    return eligiblePool.sort((left, right) => {
      const score = statusPriority[resolveEffectiveStatus(right)] - statusPriority[resolveEffectiveStatus(left)];

      if (score !== 0) {
        return score;
      }

      return right.publishedAt.localeCompare(left.publishedAt);
    })[0];
  }

  const broken = sorted.find((version) => resolveEffectiveStatus(version) === "broken");

  if (broken) {
    return broken;
  }

  return sorted[0];
}

export function everyRelevantVersionBroken(pkg: ModPackage): boolean {
  const relevant = pkg.versions.filter((version) => version.baseZone !== "red");

  return relevant.length > 0 && relevant.every((version) => resolveEffectiveStatus(version) === "broken");
}
