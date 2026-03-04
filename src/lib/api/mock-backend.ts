import { seedPackages } from "../mock-data";
import {
  compareVersionNumbers,
  currentReferenceNote,
  currentReferenceSource,
  currentReferenceState,
  everyRelevantVersionBroken,
  pickRecommendedVersion,
  resolveEffectiveStatus
} from "../status";
import type {
  CacheSummaryDto,
  CatalogSummaryDto,
  DependencySummaryItemDto,
  CreateProfileInput,
  DependencyNodeDto,
  DeleteProfileResult,
  DownloadJobDto,
  EffectiveStatus,
  GetVersionDependenciesInput,
  DependencyResolutionKind,
  InstallTaskDto,
  ListReferenceRowsInput,
  ListReferenceRowsResult,
  ModPackage,
  ModVersion,
  PackageCardDto,
  PackageDetailDto,
  ProfileDetailDto,
  ProfilesStorageSummaryDto,
  ProfileSummaryDto,
  ReferenceRow,
  ReferenceState,
  SearchPackagesInput,
  SearchPackagesResult,
  SetReferenceStateInput,
  QueueInstallToCacheInput,
  QueueInstallToCacheResult,
  SyncCatalogInput,
  SyncCatalogResult,
  UpdateProfileInput,
  UnresolvedDependencySummaryItemDto,
  VersionDependenciesDto,
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
  profiles: Array<{
    id: string;
    name: string;
    notes: string;
    gamePath: string;
    lastPlayed: string | null;
    launchModeDefault: "steam" | "direct";
    isBuiltinDefault: boolean;
  }>;
  activeProfileId: string;
  cachedVersions: Array<{
    packageId: string;
    versionId: string;
    packageName: string;
    versionLabel: string;
    fileSize: number;
  }>;
  tasks: InstallTaskDto[];
  downloads: DownloadJobDto[];
};

const STORAGE_KEY = "49modman.mock-backend.v1";

const defaultDb: MockDb = {
  warningPrefs: {
    red: true,
    broken: true
  },
  lastSyncAt: null,
  overrides: [],
  profiles: [
    {
      id: "default",
      name: "Default",
      notes: "Built-in fallback profile.",
      gamePath: "",
      lastPlayed: null,
      launchModeDefault: "steam",
      isBuiltinDefault: true
    }
  ],
  activeProfileId: "default",
  cachedVersions: [],
  tasks: [],
  downloads: []
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

function normalizeDb(db: MockDb): MockDb {
  const profiles = db.profiles.length > 0 ? db.profiles : clone(defaultDb.profiles);
  const hasActive = profiles.some((profile) => profile.id === db.activeProfileId);

  return {
    ...db,
    profiles,
    activeProfileId: hasActive ? db.activeProfileId : "default",
    cachedVersions: db.cachedVersions ?? [],
    tasks: db.tasks ?? [],
    downloads: db.downloads ?? []
  };
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

function findPackageVersion(packageId: string, versionId: string) {
  const pkg = currentPackages().find((entry) => entry.id === packageId);
  const version = pkg?.versions.find((entry) => entry.id === versionId);

  return pkg && version ? { pkg, version } : null;
}

type IndexedVersionRecord = {
  packageId: string;
  packageName: string;
  versionId: string;
  versionNumber: string;
  effectiveStatus: EffectiveStatus;
  referenceNote?: string;
  dependencies: string[];
};

type DependencyCatalogIndex = {
  versionsById: Map<string, IndexedVersionRecord>;
  versionIdByDependencyRaw: Map<string, string>;
};

type ResolvedSummaryPackageAccumulator = {
  packageId: string;
  packageName: string;
  versionId: string;
  versionNumber: string;
  effectiveStatus: EffectiveStatus;
  referenceNote?: string;
  minDepth: number;
  collapsedVersionNumbers: Set<string>;
};

function buildDependencyCatalogIndex(packages: ModPackage[]): DependencyCatalogIndex {
  const versionsById = new Map<string, IndexedVersionRecord>();
  const versionIdByDependencyRaw = new Map<string, string>();

  for (const pkg of packages) {
    for (const version of pkg.versions) {
      const record: IndexedVersionRecord = {
        packageId: pkg.id,
        packageName: pkg.fullName,
        versionId: version.id,
        versionNumber: version.versionNumber,
        effectiveStatus: version.effectiveStatus ?? resolveEffectiveStatus(version),
        referenceNote: currentReferenceNote(version),
        dependencies: [...(version.dependencies ?? [])]
      };
      const dependencyRaw = `${pkg.fullName}-${version.versionNumber}`;

      versionsById.set(version.id, record);
      if (!versionIdByDependencyRaw.has(dependencyRaw)) {
        versionIdByDependencyRaw.set(dependencyRaw, version.id);
      }
    }
  }

  return {
    versionsById,
    versionIdByDependencyRaw
  };
}

function resolveIndexedDependency(
  index: DependencyCatalogIndex,
  raw: string
): IndexedVersionRecord | undefined {
  const versionId = index.versionIdByDependencyRaw.get(raw);
  return versionId ? index.versionsById.get(versionId) : undefined;
}

function buildResolvedDependencyNode(
  raw: string,
  record: IndexedVersionRecord,
  resolution: DependencyResolutionKind,
  children: DependencyNodeDto[]
): DependencyNodeDto {
  return {
    raw,
    packageId: record.packageId,
    packageName: record.packageName,
    versionId: record.versionId,
    versionNumber: record.versionNumber,
    effectiveStatus: record.effectiveStatus,
    referenceNote: record.referenceNote,
    resolution,
    children
  };
}

function buildUnresolvedDependencyNode(raw: string): DependencyNodeDto {
  return {
    raw,
    resolution: "unresolved",
    children: []
  };
}

function collectSummaryDependency(
  index: DependencyCatalogIndex,
  raw: string,
  depth: number,
  ancestry: Set<string>,
  visitedResolved: Set<string>,
  resolvedByPackage: Map<string, ResolvedSummaryPackageAccumulator>,
  unresolvedByRaw: Map<string, UnresolvedDependencySummaryItemDto>,
  resolvedOrder: string[],
  unresolvedOrder: string[]
) {
  const normalized = raw.trim();
  if (!normalized) {
    if (!unresolvedByRaw.has(raw)) {
      unresolvedOrder.push(raw);
      unresolvedByRaw.set(raw, { raw, minDepth: depth });
    } else {
      unresolvedByRaw.get(raw)!.minDepth = Math.min(unresolvedByRaw.get(raw)!.minDepth, depth);
    }
    return;
  }

  const resolved = resolveIndexedDependency(index, normalized);
  if (!resolved) {
    if (!unresolvedByRaw.has(raw)) {
      unresolvedOrder.push(raw);
      unresolvedByRaw.set(raw, { raw, minDepth: depth });
    } else {
      unresolvedByRaw.get(raw)!.minDepth = Math.min(unresolvedByRaw.get(raw)!.minDepth, depth);
    }
    return;
  }

  const versionKey = `${resolved.packageId}:${resolved.versionId}`;
  if (ancestry.has(versionKey)) {
    return;
  }

  const existing = resolvedByPackage.get(resolved.packageId);
  if (!existing) {
    resolvedOrder.push(resolved.packageId);
    resolvedByPackage.set(resolved.packageId, {
      packageId: resolved.packageId,
      packageName: resolved.packageName,
      versionId: resolved.versionId,
      versionNumber: resolved.versionNumber,
      effectiveStatus: resolved.effectiveStatus,
      referenceNote: resolved.referenceNote,
      minDepth: depth,
      collapsedVersionNumbers: new Set<string>()
    });
  } else {
    existing.minDepth = Math.min(existing.minDepth, depth);
    const versionDelta = compareVersionNumbers(resolved.versionNumber, existing.versionNumber);
    if (versionDelta > 0) {
      existing.collapsedVersionNumbers.add(existing.versionNumber);
      existing.versionId = resolved.versionId;
      existing.versionNumber = resolved.versionNumber;
      existing.effectiveStatus = resolved.effectiveStatus;
      existing.referenceNote = resolved.referenceNote;
      existing.collapsedVersionNumbers.delete(existing.versionNumber);
    } else if (versionDelta < 0) {
      existing.collapsedVersionNumbers.add(resolved.versionNumber);
    }
  }

  if (visitedResolved.has(versionKey)) {
    return;
  }

  visitedResolved.add(versionKey);
  ancestry.add(versionKey);
  for (const dependency of resolved.dependencies) {
    collectSummaryDependency(
      index,
      dependency,
      depth + 1,
      ancestry,
      visitedResolved,
      resolvedByPackage,
      unresolvedByRaw,
      resolvedOrder,
      unresolvedOrder
    );
  }
  ancestry.delete(versionKey);
}

function buildDependencyTreeNode(
  index: DependencyCatalogIndex,
  raw: string,
  ancestry: Set<string>,
  expandedVersions: Set<string>
): DependencyNodeDto {
  const normalized = raw.trim();
  if (!normalized) {
    return buildUnresolvedDependencyNode(raw);
  }

  const resolved = resolveIndexedDependency(index, normalized);
  if (!resolved) {
    return buildUnresolvedDependencyNode(raw);
  }

  const versionKey = `${resolved.packageId}:${resolved.versionId}`;
  if (ancestry.has(versionKey)) {
    return buildResolvedDependencyNode(raw, resolved, "cycle", []);
  }

  if (expandedVersions.has(versionKey)) {
    return buildResolvedDependencyNode(raw, resolved, "repeated", []);
  }

  expandedVersions.add(versionKey);
  ancestry.add(versionKey);
  const children = resolved.dependencies.map((dependency) =>
    buildDependencyTreeNode(index, dependency, ancestry, expandedVersions)
  );
  ancestry.delete(versionKey);

  return buildResolvedDependencyNode(raw, resolved, "resolved", children);
}

function currentProfiles(): ProfileSummaryDto[] {
  const db = normalizeDb(loadDb());

  return clone(db.profiles)
    .sort((left, right) => {
      if (left.isBuiltinDefault !== right.isBuiltinDefault) {
        return left.isBuiltinDefault ? -1 : 1;
      }

      return left.name.localeCompare(right.name);
    })
    .map((profile) => ({
      ...profile,
      installedCount: 0,
      enabledCount: 0,
      profileSizeBytes: 1024
    }));
}

function findActiveProfile(): ProfileDetailDto | null {
  const db = normalizeDb(loadDb());
  const profile = db.profiles.find((entry) => entry.id === db.activeProfileId);

  if (!profile) {
    return null;
  }

  return {
    ...clone(profile),
    installedMods: []
  };
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
        recommendedVersionId: recommended.id,
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

export async function searchPackagesMock(input: SearchPackagesInput): Promise<SearchPackagesResult> {
  const pageSize = Math.max(1, input.pageSize ?? 40);
  const cursor = Math.max(0, input.cursor ?? 0);
  const cards = searchPackagesInternal(input);
  const window = cards.slice(cursor, cursor + pageSize + 1);
  const hasMore = window.length > pageSize;

  return {
    items: hasMore ? window.slice(0, pageSize) : window,
    nextCursor: hasMore ? cursor + pageSize : null,
    hasMore,
    pageSize
  };
}

export async function getPackageDetailMock(packageId: string): Promise<PackageDetailDto | null> {
  return currentPackages().find((pkg) => pkg.id === packageId) ?? null;
}

export async function getVersionDependenciesMock(
  input: GetVersionDependenciesInput
): Promise<VersionDependenciesDto> {
  const packages = currentPackages();
  const resolved = findPackageVersion(input.packageId, input.versionId);

  if (!resolved) {
    throw new Error("That package version is not available in the cached Thunderstore catalog.");
  }

  const index = buildDependencyCatalogIndex(packages);
  const rootKey = `${resolved.pkg.id}:${resolved.version.id}`;
  const summaryAncestry = new Set([rootKey]);
  const visitedResolved = new Set<string>();
  const resolvedByPackage = new Map<string, ResolvedSummaryPackageAccumulator>();
  const unresolvedByRaw = new Map<string, UnresolvedDependencySummaryItemDto>();
  const resolvedOrder: string[] = [];
  const unresolvedOrder: string[] = [];

  for (const dependency of resolved.version.dependencies ?? []) {
    collectSummaryDependency(
      index,
      dependency,
      1,
      summaryAncestry,
      visitedResolved,
      resolvedByPackage,
      unresolvedByRaw,
      resolvedOrder,
      unresolvedOrder
    );
  }

  const treeAncestry = new Set([rootKey]);
  const expandedVersions = new Set<string>();

  return {
    rootPackageId: resolved.pkg.id,
    rootPackageName: resolved.pkg.fullName,
    rootVersionId: resolved.version.id,
    rootVersionNumber: resolved.version.versionNumber,
    summary: {
      direct: resolvedOrder
        .map((packageId) => resolvedByPackage.get(packageId))
        .filter(
          (item): item is ResolvedSummaryPackageAccumulator => Boolean(item && item.minDepth <= 1)
        )
        .map(
          (item): DependencySummaryItemDto => ({
            packageId: item.packageId,
            packageName: item.packageName,
            versionId: item.versionId,
            versionNumber: item.versionNumber,
            effectiveStatus: item.effectiveStatus,
            referenceNote: item.referenceNote,
            minDepth: item.minDepth,
            collapsedVersionNumbers: [...item.collapsedVersionNumbers].sort((left, right) =>
              compareVersionNumbers(right, left)
            )
          })
        ),
      transitive: resolvedOrder
        .map((packageId) => resolvedByPackage.get(packageId))
        .filter(
          (item): item is ResolvedSummaryPackageAccumulator => Boolean(item && item.minDepth >= 2)
        )
        .map(
          (item): DependencySummaryItemDto => ({
            packageId: item.packageId,
            packageName: item.packageName,
            versionId: item.versionId,
            versionNumber: item.versionNumber,
            effectiveStatus: item.effectiveStatus,
            referenceNote: item.referenceNote,
            minDepth: item.minDepth,
            collapsedVersionNumbers: [...item.collapsedVersionNumbers].sort((left, right) =>
              compareVersionNumbers(right, left)
            )
          })
        ),
      unresolved: unresolvedOrder
        .map((raw) => unresolvedByRaw.get(raw))
        .filter((item): item is UnresolvedDependencySummaryItemDto => Boolean(item))
    },
    treeItems: (resolved.version.dependencies ?? []).map((dependency) =>
      buildDependencyTreeNode(index, dependency, treeAncestry, expandedVersions)
    )
  };
}

export async function warmDependencyIndexMock(): Promise<void> {
  return;
}

export async function listProfilesMock(): Promise<ProfileSummaryDto[]> {
  const db = normalizeDb(loadDb());
  saveDb(db);
  return currentProfiles();
}

export async function getActiveProfileMock(): Promise<ProfileDetailDto | null> {
  const db = normalizeDb(loadDb());
  saveDb(db);
  return findActiveProfile();
}

export async function setActiveProfileMock(profileId: string): Promise<ProfileDetailDto | null> {
  const db = normalizeDb(loadDb());

  if (!db.profiles.some((profile) => profile.id === profileId)) {
    throw new Error(`Profile ${profileId} does not exist.`);
  }

  db.activeProfileId = profileId;
  saveDb(db);
  return findActiveProfile();
}

export async function createProfileMock(input: CreateProfileInput): Promise<ProfileDetailDto> {
  const db = normalizeDb(loadDb());
  const name = input.name.trim();

  if (!name) {
    throw new Error("Profile name cannot be empty.");
  }

  if (db.profiles.some((profile) => profile.name.toLowerCase() === name.toLowerCase())) {
    throw new Error("A profile with that name already exists.");
  }

  const profile = {
    id: `profile-${Date.now()}`,
    name,
    notes: input.notes ?? "",
    gamePath: input.gamePath ?? "",
    lastPlayed: null,
    launchModeDefault: input.launchModeDefault ?? "steam",
    isBuiltinDefault: false
  } as const;

  db.profiles = [profile, ...db.profiles];
  db.activeProfileId = profile.id;
  saveDb(db);

  return {
    ...profile,
    installedMods: []
  };
}

export async function updateProfileMock(input: UpdateProfileInput): Promise<ProfileDetailDto> {
  const db = normalizeDb(loadDb());
  const profile = db.profiles.find((entry) => entry.id === input.profileId);

  if (!profile) {
    throw new Error("That profile does not exist.");
  }

  const name = input.name.trim();
  if (!name) {
    throw new Error("Profile name cannot be empty.");
  }

  if (
    db.profiles.some(
      (entry) => entry.id !== input.profileId && entry.name.toLowerCase() === name.toLowerCase()
    )
  ) {
    throw new Error("A profile with that name already exists.");
  }

  if (profile.isBuiltinDefault && name !== "Default") {
    throw new Error("The built-in Default profile name cannot be changed.");
  }

  Object.assign(profile, {
    name,
    notes: input.notes ?? "",
    gamePath: input.gamePath ?? "",
    launchModeDefault: input.launchModeDefault ?? "steam"
  });

  saveDb(db);

  return {
    ...clone(profile),
    installedMods: []
  };
}

export async function deleteProfileMock(profileId: string): Promise<DeleteProfileResult> {
  const db = normalizeDb(loadDb());
  const profile = db.profiles.find((entry) => entry.id === profileId);

  if (!profile) {
    throw new Error("That profile does not exist.");
  }

  if (profile.isBuiltinDefault) {
    throw new Error("The built-in Default profile cannot be deleted.");
  }

  db.profiles = db.profiles.filter((entry) => entry.id !== profileId);

  if (db.activeProfileId === profileId) {
    db.activeProfileId = "default";
  }

  saveDb(db);

  return {
    deletedId: profileId,
    nextActiveProfileId: db.activeProfileId
  };
}

export async function getProfileDetailMock(profileId: string): Promise<ProfileDetailDto | null> {
  const db = normalizeDb(loadDb());
  saveDb(db);
  const profile = db.profiles.find((entry) => entry.id === profileId);

  return profile
    ? {
        ...clone(profile),
        installedMods: []
      }
    : null;
}

export async function resetAllDataMock(): Promise<void> {
  saveDb(clone(defaultDb));
}

export async function openProfilesFolderMock(): Promise<void> {
  return;
}

export async function openActiveProfileFolderMock(): Promise<void> {
  return;
}

export async function getProfilesStorageSummaryMock(): Promise<ProfilesStorageSummaryDto> {
  const db = normalizeDb(loadDb());
  const profileCount = db.profiles.length;

  // Browser mock has no real profile filesystem; provide deterministic pseudo totals.
  return {
    profileCount,
    profilesTotalBytes: profileCount * 1024,
    activeProfileBytes: 1024
  };
}

function taskForVersion(db: MockDb, versionId: string): InstallTaskDto | undefined {
  return db.tasks.find(
    (task) => task.kind === "cache_version" && task.detail === versionId && (task.status === "queued" || task.status === "running")
  );
}

function finishMockTask(taskId: string, cached: boolean) {
  const db = normalizeDb(loadDb());
  const task = db.tasks.find((entry) => entry.id === taskId);
  const download = db.downloads.find((entry) => entry.taskId === taskId);

  if (!task || !download) {
    return;
  }

  task.status = "succeeded";
  task.progressStep = "finalizing";
  task.progressCurrent = 4;
  task.progressTotal = 4;
  task.finishedAt = nowIso();

  download.status = cached ? "cached" : "verifying";
  download.cacheHit = cached;
  download.progressLabel = cached ? "Already cached locally" : "Archive cached successfully";
  download.updatedAt = nowIso();

  saveDb(db);

  window.setTimeout(() => {
    const nextDb = normalizeDb(loadDb());
    nextDb.downloads = nextDb.downloads.filter((entry) => entry.taskId !== taskId);
    saveDb(nextDb);
  }, 800);
}

export async function queueInstallToCacheMock(
  input: QueueInstallToCacheInput
): Promise<QueueInstallToCacheResult> {
  const db = normalizeDb(loadDb());
  const pkg = currentPackages().find((entry) => entry.id === input.packageId);
  const version = pkg?.versions.find((entry) => entry.id === input.versionId);

  if (!pkg || !version) {
    throw new Error("That package version is not available in the cached Thunderstore catalog.");
  }

  const existingTask = taskForVersion(db, input.versionId);
  if (existingTask) {
    return {
      taskId: existingTask.id
    };
  }

  const taskId = `task-${Date.now()}`;
  const cached = db.cachedVersions.some((entry) => entry.versionId === input.versionId);
  db.tasks.unshift({
    id: taskId,
    kind: "cache_version",
    status: "running",
    title: `Caching ${pkg.fullName} ${version.versionNumber}`,
    detail: input.versionId,
    progressStep: cached ? "checking_cache" : "downloading",
    progressCurrent: cached ? 1 : 2,
    progressTotal: 4,
    createdAt: nowIso(),
    startedAt: nowIso()
  });
  db.downloads.unshift({
    id: `job-${Date.now()}`,
    taskId,
    packageName: pkg.fullName,
    versionLabel: version.versionNumber,
    sourceKind: "thunderstore",
    status: cached ? "checking_cache" : "downloading",
    cacheHit: false,
    bytesDownloaded: cached ? 0 : 524288,
    totalBytes: cached ? 0 : 1048576,
    speedBps: cached ? undefined : 262144,
    progressLabel: cached ? "Checking the shared cache" : "Downloading from Thunderstore",
    updatedAt: nowIso()
  });
  saveDb(db);

  if (cached) {
    window.setTimeout(() => finishMockTask(taskId, true), 120);
  } else {
    window.setTimeout(() => {
      const nextDb = normalizeDb(loadDb());
      const job = nextDb.downloads.find((entry) => entry.taskId === taskId);
      if (job) {
        job.bytesDownloaded = job.totalBytes ?? job.bytesDownloaded;
        job.progressLabel = "Verifying cached archive";
        job.status = "verifying";
        job.updatedAt = nowIso();
      }
      if (!nextDb.cachedVersions.some((entry) => entry.versionId === input.versionId)) {
        nextDb.cachedVersions.push({
          packageId: input.packageId,
          versionId: input.versionId,
          packageName: pkg.fullName,
          versionLabel: version.versionNumber,
          fileSize: 1048576
        });
      }
      saveDb(nextDb);
    }, 450);
    window.setTimeout(() => finishMockTask(taskId, false), 900);
  }

  return {
    taskId
  };
}

export async function getCacheSummaryMock(): Promise<CacheSummaryDto> {
  const db = normalizeDb(loadDb());
  return {
    archiveCount: db.cachedVersions.length,
    totalBytes: db.cachedVersions.reduce((sum, entry) => sum + entry.fileSize, 0),
    cachePath: "/mock/cache",
    hasActiveDownloads: db.tasks.some((task) => task.status === "queued" || task.status === "running")
  };
}

export async function openCacheFolderMock(): Promise<void> {
  return;
}

export async function clearCacheMock(): Promise<CacheSummaryDto> {
  const db = normalizeDb(loadDb());
  if (db.tasks.some((task) => task.status === "queued" || task.status === "running")) {
    throw new Error("Cannot clear the cache while downloads are active.");
  }

  db.cachedVersions = [];
  db.downloads = db.downloads.filter((entry) => {
    const task = db.tasks.find((taskEntry) => taskEntry.id === entry.taskId);
    return task?.status === "failed";
  });
  db.tasks = db.tasks.filter((task) => task.status === "failed");
  saveDb(db);
  return getCacheSummaryMock();
}

export async function listActiveDownloadsMock(): Promise<DownloadJobDto[]> {
  return normalizeDb(loadDb()).downloads;
}

export async function getTaskMock(taskId: string): Promise<InstallTaskDto | null> {
  return normalizeDb(loadDb()).tasks.find((entry) => entry.id === taskId) ?? null;
}

export async function listReferenceRowsMock(input: ListReferenceRowsInput): Promise<ListReferenceRowsResult> {
  const pageSize = Math.max(1, input.pageSize ?? 50);
  const cursor = Math.max(0, input.cursor ?? 0);
  const rows = referenceRowsInternal(input.query);
  const window = rows.slice(cursor, cursor + pageSize + 1);
  const hasMore = window.length > pageSize;

  return {
    items: hasMore ? window.slice(0, pageSize) : window,
    nextCursor: hasMore ? cursor + pageSize : null,
    hasMore,
    pageSize
  };
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
