export type AppView = "overview" | "browse" | "profiles" | "downloads" | "settings";

export type BaseZone = "orange" | "green" | "yellow" | "red";
export type ReferenceState = "verified" | "broken" | "neutral";
export type EffectiveStatus = "broken" | "verified" | BaseZone;

export type LaunchMode = "steam" | "direct";
export type LaunchVariant = "vanilla" | "modded";

export interface ModVersion {
  id: string;
  versionNumber: string;
  publishedAt: string;
  downloads: number;
  baseZone: BaseZone;
  bundledReferenceState?: Exclude<ReferenceState, "neutral">;
  bundledReferenceNote?: string;
  overrideReferenceState?: ReferenceState;
  overrideReferenceNote?: string;
  effectiveStatus?: EffectiveStatus;
  referenceSource?: "bundled" | "override";
}

export interface ModPackage {
  id: string;
  fullName: string;
  author: string;
  summary: string;
  categories: string[];
  totalDownloads: number;
  rating: number;
  websiteUrl: string;
  versions: ModVersion[];
}

export interface PackageCardDto {
  id: string;
  fullName: string;
  author: string;
  summary: string;
  categories: string[];
  totalDownloads: number;
  rating: number;
  versionCount: number;
  recommendedVersion: string;
  effectiveStatus: EffectiveStatus;
  everyRelevantVersionBroken: boolean;
}

export type PackageDetailDto = ModPackage;
export type PackageVersionDto = ModVersion;

export interface CatalogSummaryDto {
  hasCatalog: boolean;
  packageCount: number;
  versionCount: number;
  lastSyncLabel: string;
}

export interface SyncCatalogResult {
  outcome: "synced" | "skipped";
  packageCount: number;
  versionCount: number;
  syncedAt: string | null;
  message: string;
}

export interface InstalledMod {
  packageId: string;
  versionId: string;
  enabled: boolean;
}

export interface Profile {
  id: string;
  name: string;
  gamePath: string;
  lastPlayed: string;
  launchModeDefault: LaunchMode;
  notes: string;
  installedMods: InstalledMod[];
}

export interface DownloadItem {
  id: string;
  packageName: string;
  versionNumber: string;
  progressLabel: string;
  status: "cached" | "active" | "queued" | "failed";
  speedLabel: string;
  cacheHit: boolean;
}

export interface WarningModalState {
  packageId: string;
  versionId: string;
  status: "red" | "broken";
}

export interface ActivityItem {
  id: string;
  title: string;
  detail: string;
  tone: "neutral" | "positive" | "warning";
}

export interface ReferenceRow {
  packageId: string;
  packageName: string;
  versionId: string;
  versionNumber: string;
  publishedAt: string;
  baseZone: BaseZone;
  effectiveStatus: EffectiveStatus;
  referenceSource?: "bundled" | "override";
  referenceState?: Exclude<ReferenceState, "neutral">;
  note?: string;
}

export interface WarningPrefsDto {
  red: boolean;
  broken: boolean;
}

export interface AppState {
  view: AppView;
  browseSearchDraft: string;
  browseSearchSubmitted: string;
  visibleStatuses: EffectiveStatus[];
  selectedPackageId: string;
  selectedProfileId: string;
  packages: ModPackage[];
  profiles: Profile[];
  downloads: DownloadItem[];
  activities: ActivityItem[];
  warningPrefs: WarningPrefsDto;
  modal: WarningModalState | null;
  referenceSearchDraft: string;
  referenceSearchSubmitted: string;
  isRefreshingCatalog: boolean;
  isBootstrapping: boolean;
  isLoadingPackageDetail: boolean;
  isLoadingReferences: boolean;
  lastCatalogRefreshLabel: string;
  catalogCards: PackageCardDto[];
  selectedPackageDetail?: PackageDetailDto;
  referenceRowsData: ReferenceRow[];
  catalogError: string | null;
  referenceError: string | null;
  settingsError: string | null;
}

export interface SyncCatalogInput {
  force?: boolean;
}

export interface SearchPackagesInput {
  query: string;
  visibleStatuses: EffectiveStatus[];
}

export interface SetReferenceStateInput {
  packageId: string;
  versionId: string;
  referenceState: ReferenceState;
}
