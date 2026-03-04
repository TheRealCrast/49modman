import { seedPackages } from "../mock-data";
import {
  currentReferenceNote,
  currentReferenceSource,
  currentReferenceState,
  everyRelevantVersionBroken,
  pickRecommendedVersion,
  resolveEffectiveStatus
} from "../status";
import type {
  CatalogSummaryDto,
  EffectiveStatus,
  ModPackage,
  ModVersion,
  PackageCardDto,
  PackageDetailDto,
  ReferenceRow,
  ReferenceState,
  SearchPackagesInput,
  SetReferenceStateInput,
  SyncCatalogInput,
  SyncCatalogResult,
  WarningPrefsDto
} from "../types";

type StoredOverride = {
  packageId: string;
  versionId: string;
  referenceState: ReferenceState;
  note?: string;
};

type MockDb = {
  warningPrefs: WarningPrefsDto;
  lastSyncAt: string | null;
  overrides: StoredOverride[];
};

const STORAGE_KEY = "49modman.mock-backend.v1";

const defaultDb: MockDb = {
  warningPrefs: {
    red: true,
    broken: true
  },
  lastSyncAt: null,
  overrides: []
};

function loadDb(): MockDb {
  if (typeof localStorage === "undefined") {
    return defaultDb;
  }

  try {
    const raw = localStorage.getItem(STORAGE_KEY);

    if (!raw) {
      return defaultDb;
    }

    return {
      ...defaultDb,
      ...JSON.parse(raw)
    };
  } catch {
    return defaultDb;
  }
}

function saveDb(db: MockDb) {
  if (typeof localStorage === "undefined") {
    return;
  }

  localStorage.setItem(STORAGE_KEY, JSON.stringify(db));
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function applyOverrides(packages: ModPackage[]): ModPackage[] {
  const db = loadDb();
  const overridesByKey = new Map(
    db.overrides.map((entry) => [`${entry.packageId}:${entry.versionId}`, entry])
  );

  return clone(packages).map((pkg) => ({
    ...pkg,
    versions: pkg.versions.map((version) => {
      const override = overridesByKey.get(`${pkg.id}:${version.id}`);

      if (!override) {
        return {
          ...version,
          effectiveStatus: resolveEffectiveStatus(version),
          referenceSource: currentReferenceSource(version)
        };
      }

      const nextVersion: ModVersion = {
        ...version,
        overrideReferenceState: override.referenceState,
        overrideReferenceNote: override.note,
      };

      return {
        ...nextVersion,
        effectiveStatus: resolveEffectiveStatus(nextVersion),
        referenceSource: currentReferenceSource(nextVersion)
      };
    })
  }));
}

function currentPackages(): ModPackage[] {
  return applyOverrides(seedPackages);
}

function nowIso(): string {
  return new Date().toISOString();
}

function searchPackagesInternal(input: SearchPackagesInput): PackageCardDto[] {
  const query = input.query.trim().toLowerCase();

  return currentPackages()
    .map((pkg) => {
      const recommended = pickRecommendedVersion(pkg);

      return {
        id: pkg.id,
        fullName: pkg.fullName,
        author: pkg.author,
        summary: pkg.summary,
        categories: pkg.categories,
        totalDownloads: pkg.totalDownloads,
        rating: pkg.rating,
        versionCount: pkg.versions.length,
        recommendedVersion: recommended.versionNumber,
        effectiveStatus: resolveEffectiveStatus(recommended),
        everyRelevantVersionBroken: everyRelevantVersionBroken(pkg)
      };
    })
    .filter((card) => input.visibleStatuses.includes(card.effectiveStatus))
    .filter((card) => {
      if (!query) {
        return true;
      }

      return [card.fullName, card.author, card.summary, ...card.categories]
        .join(" ")
        .toLowerCase()
        .includes(query);
    })
    .sort((left, right) => {
      const priority: Record<EffectiveStatus, number> = {
        verified: 5,
        green: 4,
        yellow: 3,
        orange: 2,
        red: 1,
        broken: 0
      };
      const score = priority[right.effectiveStatus] - priority[left.effectiveStatus];

      if (score !== 0) {
        return score;
      }

      return right.totalDownloads - left.totalDownloads;
    });
}

function referenceRowsInternal(query: string): ReferenceRow[] {
  const search = query.trim().toLowerCase();
  const priority: Record<EffectiveStatus, number> = {
    verified: 5,
    green: 4,
    yellow: 3,
    orange: 2,
    red: 1,
    broken: 0
  };

  return currentPackages()
    .flatMap((pkg) =>
      pkg.versions.map((version) => ({
        packageId: pkg.id,
        packageName: pkg.fullName,
        versionId: version.id,
        versionNumber: version.versionNumber,
        publishedAt: version.publishedAt,
        baseZone: version.baseZone,
        effectiveStatus: resolveEffectiveStatus(version),
        referenceSource: currentReferenceSource(version),
        referenceState: currentReferenceState(version),
        note: currentReferenceNote(version)
      }))
    )
    .filter((row) => {
      if (!search) {
        return true;
      }

      return [row.packageName, row.versionNumber, row.note ?? "", row.effectiveStatus]
        .join(" ")
        .toLowerCase()
        .includes(search);
    })
    .sort((left, right) => {
      const statusDelta = priority[right.effectiveStatus] - priority[left.effectiveStatus];

      if (statusDelta !== 0) {
        return statusDelta;
      }

      return right.publishedAt.localeCompare(left.publishedAt);
    });
}

export async function syncCatalogMock(input: SyncCatalogInput = {}): Promise<SyncCatalogResult> {
  const db = loadDb();
  const alreadySynced = Boolean(db.lastSyncAt);

  if (alreadySynced && !input.force) {
    return {
      outcome: "skipped",
      packageCount: seedPackages.length,
      versionCount: seedPackages.reduce((count, pkg) => count + pkg.versions.length, 0),
      syncedAt: db.lastSyncAt,
      message: "Cached mod list ready"
    };
  }

  db.lastSyncAt = nowIso();
  saveDb(db);

  return {
    outcome: "synced",
    packageCount: seedPackages.length,
    versionCount: seedPackages.reduce((count, pkg) => count + pkg.versions.length, 0),
    syncedAt: db.lastSyncAt,
    message: "Cache refreshed just now"
  };
}

export async function getCatalogSummaryMock(): Promise<CatalogSummaryDto> {
  const db = loadDb();

  return {
    hasCatalog: Boolean(db.lastSyncAt),
    packageCount: seedPackages.length,
    versionCount: seedPackages.reduce((count, pkg) => count + pkg.versions.length, 0),
    lastSyncLabel: db.lastSyncAt ? "Cached mod list ready" : "Catalog not synced yet"
  };
}

export async function searchPackagesMock(input: SearchPackagesInput): Promise<PackageCardDto[]> {
  return searchPackagesInternal(input);
}

export async function getPackageDetailMock(packageId: string): Promise<PackageDetailDto | null> {
  return currentPackages().find((pkg) => pkg.id === packageId) ?? null;
}

export async function listReferenceRowsMock(query: string): Promise<ReferenceRow[]> {
  return referenceRowsInternal(query);
}

export async function setReferenceStateMock(input: SetReferenceStateInput): Promise<ReferenceRow> {
  const db = loadDb();
  const nextOverride = db.overrides.filter(
    (entry) => !(entry.packageId === input.packageId && entry.versionId === input.versionId)
  );

  if (input.referenceState !== "neutral") {
    nextOverride.push({
      packageId: input.packageId,
      versionId: input.versionId,
      referenceState: input.referenceState,
      note:
        input.referenceState === "verified"
          ? "Locally marked verified from the prototype reference editor."
          : "Locally marked broken from the prototype reference editor."
    });
  } else {
    nextOverride.push({
      packageId: input.packageId,
      versionId: input.versionId,
      referenceState: "neutral"
    });
  }

  db.overrides = nextOverride;
  saveDb(db);

  const row = referenceRowsInternal("")
    .find((entry) => entry.packageId === input.packageId && entry.versionId === input.versionId);

  if (!row) {
    throw new Error(`Reference row not found for ${input.packageId}:${input.versionId}`);
  }

  return row;
}

export async function getWarningPrefsMock(): Promise<WarningPrefsDto> {
  return loadDb().warningPrefs;
}

export async function setWarningPreferenceMock(
  kind: "red" | "broken",
  enabled: boolean
): Promise<WarningPrefsDto> {
  const db = loadDb();
  db.warningPrefs = {
    ...db.warningPrefs,
    [kind]: enabled
  };
  saveDb(db);
  return db.warningPrefs;
}
