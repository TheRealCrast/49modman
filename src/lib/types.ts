export type AppView = "overview" | "browse" | "profiles" | "downloads" | "settings";
export type RuntimeKind = "tauri" | "browser-mock";

export type BaseZone = "orange" | "green" | "yellow" | "red";
export type ReferenceState = "verified" | "broken" | "neutral";
export type EffectiveStatus = "broken" | "verified" | BaseZone;
export type BrowseSortMode =
  | "mostDownloads"
  | "compatibility"
  | "lastUpdated"
  | "nameAsc"
  | "nameDesc";

export type LaunchMode = "steam" | "direct";
export type LaunchVariant = "vanilla" | "modded";

export interface SteamScanResult {
  steamRootPaths: string[];
  libraryPaths: string[];
  gamePaths: string[];
  selectedGamePath: string | null;
}

export interface ValidateV49InstallInput {
  gamePathOverride?: string;
  profileId?: string;
}

export interface LaunchProfileInput {
  profileId: string;
  launchMode: LaunchMode;
  gamePathOverride?: string;
  protonRuntimeId?: string;
}

export interface LaunchVanillaInput {
  launchMode: LaunchMode;
  gamePathOverride?: string;
  protonRuntimeId?: string;
}

export interface V49ValidationCheck {
  key: string;
  ok: boolean;
  code: string;
  message: string;
  detail?: string;
}

export interface V49ValidationResult {
  ok: boolean;
  code: string;
  message: string;
  resolvedGamePath: string | null;
  resolvedFrom: string | null;
  selectedProfileId: string | null;
  checks: V49ValidationCheck[];
  detectedExecutableSha256?: string;
  hardlinkSupported?: boolean;
}

export interface LaunchResult {
  ok: boolean;
  code: string;
  message: string;
  pid?: number;
  usedGamePath?: string;
  usedProfileId?: string;
  usedLaunchMode?: LaunchMode;
  diagnosticsPath?: string;
}

export interface ProtonRuntime {
  id: string;
  displayName: string;
  path: string;
  source: "steam" | "custom";
  isValid: boolean;
}

export interface LaunchFeedbackState {
  tone: "neutral" | "positive" | "warning";
  title: string;
  detail: string;
  diagnosticsPath?: string;
  canRepair: boolean;
}

export interface BuildRuntimeStageInput {
  profileId?: string;
}

export interface ActivateProfileInput {
  profileId?: string;
  gamePathOverride?: string;
}

export interface RuntimeStageSourceMod {
  packageId: string;
  packageName: string;
  versionId: string;
  versionNumber: string;
  installDir: string;
}

export interface RuntimeStageBuildResult {
  profileId: string;
  stagePath: string;
  mergedModCount: number;
  copiedFileCount: number;
  overwrittenFileCount: number;
  sourceMods: RuntimeStageSourceMod[];
}

export interface ActivationApplyResult {
  ok: boolean;
  code: string;
  message: string;
  profileId: string;
  gamePath: string;
  stagePath: string;
  manifestPath: string;
  cleanedPreviousActivation: boolean;
  fileCount: number;
  dirCount: number;
}

export interface VanillaCleanupResult {
  ok: boolean;
  code: string;
  message: string;
  manifestPath: string | null;
  gamePath: string | null;
  removedFileCount: number;
  removedDirCount: number;
  missingEntryCount: number;
  remainingEntryCount: number;
}

export interface ModVersion {
  id: string;
  versionNumber: string;
  publishedAt: string;
  downloads: number;
  baseZone: BaseZone;
  dependencies?: string[];
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
  recommendedVersionId: string;
  recommendedVersion: string;
  effectiveStatus: EffectiveStatus;
  everyRelevantVersionBroken: boolean;
}

export interface SearchPackagesResult {
  items: PackageCardDto[];
  nextCursor: number | null;
  hasMore: boolean;
  pageSize: number;
}

export interface ListReferenceRowsInput {
  query: string;
  cursor?: number | null;
  pageSize?: number;
}

export interface ListReferenceRowsResult {
  items: ReferenceRow[];
  nextCursor: number | null;
  hasMore: boolean;
  pageSize: number;
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

export interface ProfileInstalledModDto {
  packageId: string;
  packageName: string;
  versionId: string;
  versionNumber: string;
  enabled: boolean;
  sourceKind: string;
  installDir: string;
  installedAt: string;
  iconDataUrl?: string;
}

export interface ProfileSummaryDto {
  id: string;
  name: string;
  notes: string;
  gamePath: string;
  lastPlayed: string | null;
  launchModeDefault: LaunchMode;
  installedCount: number;
  enabledCount: number;
  isBuiltinDefault: boolean;
  profileSizeBytes: number;
}

export interface ProfileDetailDto {
  id: string;
  name: string;
  notes: string;
  gamePath: string;
  lastPlayed: string | null;
  launchModeDefault: LaunchMode;
  isBuiltinDefault: boolean;
  installedMods: ProfileInstalledModDto[];
}

export interface CreateProfileInput {
  name: string;
  notes?: string;
  gamePath?: string;
  launchModeDefault?: LaunchMode;
}

export interface UpdateProfileInput {
  profileId: string;
  name: string;
  notes?: string;
  gamePath?: string;
  launchModeDefault?: LaunchMode;
}

export interface SetInstalledModEnabledInput {
  profileId: string;
  packageId: string;
  versionId: string;
  enabled: boolean;
}

export interface UninstallInstalledModInput {
  profileId: string;
  packageId: string;
  versionId: string;
}

export interface GetUninstallDependantsInput {
  profileId: string;
  packageId: string;
  versionIds: string[];
}

export interface UninstallDependantDto {
  packageId: string;
  packageName: string;
  versionId: string;
  versionNumber: string;
  minDepth: number;
}

export interface DeleteProfileResult {
  deletedId: string;
  nextActiveProfileId: string | null;
}

export interface CacheSummaryDto {
  archiveCount: number;
  totalBytes: number;
  cachePath: string;
  hasActiveDownloads: boolean;
}

export interface CachePruneCandidateDto {
  packageId: string;
  packageName: string;
  versionId: string;
  versionNumber: string;
  archiveName: string;
  fileSize: number;
}

export interface CachePrunePreviewDto {
  removableCount: number;
  removableBytes: number;
  candidates: CachePruneCandidateDto[];
}

export interface ProfilesStorageSummaryDto {
  profileCount: number;
  profilesTotalBytes: number;
  activeProfileBytes: number;
}

export interface InstallTaskDto {
  id: string;
  kind: "cache_version";
  status: "queued" | "running" | "succeeded" | "failed";
  title: string;
  detail: string;
  progressStep?: "queued" | "checking_cache" | "downloading" | "verifying" | "finalizing";
  progressCurrent: number;
  progressTotal: number;
  errorMessage?: string;
  createdAt: string;
  startedAt?: string;
  finishedAt?: string;
}

export interface DownloadJobDto {
  id: string;
  packageName: string;
  versionLabel: string;
  taskId: string;
  sourceKind: "thunderstore";
  progressLabel: string;
  status: "queued" | "checking_cache" | "downloading" | "verifying" | "cached" | "failed";
  bytesDownloaded: number;
  totalBytes?: number;
  speedBps?: number;
  cacheHit: boolean;
  errorMessage?: string;
  updatedAt: string;
}

export interface QueueInstallToCacheInput {
  packageId: string;
  versionId: string;
}

export interface QueueInstallToCacheResult {
  taskId: string;
}

export interface InstallRequest {
  packageId: string;
  packageName: string;
  versionId: string;
  versionNumber: string;
  effectiveStatus: EffectiveStatus;
  referenceNote?: string;
}

export interface InstallActionOptions {
  includeDependencies?: boolean;
}

export type DependencyResolutionKind = "resolved" | "unresolved" | "cycle" | "repeated";

export interface DependencyNodeDto {
  raw: string;
  packageId?: string;
  packageName?: string;
  versionId?: string;
  versionNumber?: string;
  effectiveStatus?: EffectiveStatus;
  referenceNote?: string;
  resolution: DependencyResolutionKind;
  children: DependencyNodeDto[];
}

export interface DependencySummaryItemDto {
  packageId: string;
  packageName: string;
  versionId: string;
  versionNumber: string;
  effectiveStatus: EffectiveStatus;
  referenceNote?: string;
  minDepth: number;
  collapsedVersionNumbers: string[];
}

export interface UnresolvedDependencySummaryItemDto {
  raw: string;
  minDepth: number;
}

export interface DependencySummaryDto {
  direct: DependencySummaryItemDto[];
  transitive: DependencySummaryItemDto[];
  unresolved: UnresolvedDependencySummaryItemDto[];
}

export interface VersionDependenciesDto {
  rootPackageId: string;
  rootPackageName: string;
  rootVersionId: string;
  rootVersionNumber: string;
  summary: DependencySummaryDto;
  treeItems: DependencyNodeDto[];
}

export interface GetVersionDependenciesInput {
  packageId: string;
  versionId: string;
}

export interface WarningModalState {
  packageId: string;
  packageName: string;
  versionId: string;
  versionNumber: string;
  status: "red" | "broken";
  referenceNote?: string;
  switchFromVersionIds?: string[];
}

export interface DependencyModalState {
  packageId: string;
  packageName: string;
  versionId: string;
  versionNumber: string;
  isLoading: boolean;
  data?: VersionDependenciesDto;
  error?: string | null;
}

export interface FocusedVersionState {
  packageId: string;
  versionId: string;
  highlightToken: number;
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
  installWithoutDependencies: boolean;
  uninstallWithDependants: boolean;
}

export interface UninstallDependantsModalState {
  packageId: string;
  packageName: string;
  versionIds: string[];
  dependants: UninstallDependantDto[];
}

export type ResetProgressStep = "deleting" | "restoring" | "browse" | "finalizing";

export interface ResetProgressState {
  title: string;
  message: string;
  step: ResetProgressStep;
}

export interface AppState {
  view: AppView;
  runtimeKind: RuntimeKind;
  browseSearchDraft: string;
  browseSearchSubmitted: string;
  browseSortMode: BrowseSortMode;
  visibleStatuses: EffectiveStatus[];
  selectedPackageId: string;
  selectedProfileId: string;
  packages: ModPackage[];
  profiles: ProfileSummaryDto[];
  activeProfile?: ProfileDetailDto;
  downloads: DownloadJobDto[];
  cacheSummary?: CacheSummaryDto;
  clearUnreferencedCacheModal: CachePrunePreviewDto | null;
  profilesStorageSummary?: ProfilesStorageSummaryDto;
  activeCacheTaskIds: string[];
  busyPackageIds: string[];
  activities: ActivityItem[];
  protonRuntimes: ProtonRuntime[];
  selectedProtonRuntimeId: string | null;
  isLoadingProtonRuntimes: boolean;
  isLaunching: boolean;
  launchingVariant: LaunchVariant | null;
  launchFeedback: LaunchFeedbackState | null;
  warningPrefs: WarningPrefsDto;
  modal: WarningModalState | null;
  uninstallDependantsModal: UninstallDependantsModalState | null;
  resetProgress: ResetProgressState | null;
  dependencyModal: DependencyModalState | null;
  focusedVersion: FocusedVersionState | null;
  referenceSearchDraft: string;
  referenceSearchSubmitted: string;
  isRefreshingCatalog: boolean;
  isBootstrapping: boolean;
  isCatalogOverlayVisible: boolean;
  catalogOverlayTitle: string | null;
  catalogOverlayMessage: string | null;
  catalogOverlayStep: "network" | "cache" | "browse" | "dependencies" | null;
  isLoadingCatalogFirstPage: boolean;
  isLoadingCatalogNextPage: boolean;
  isLoadingPackageDetail: boolean;
  isLoadingProfiles: boolean;
  isLoadingDownloads: boolean;
  isLoadingCacheSummary: boolean;
  isLoadingProfilesStorageSummary: boolean;
  isLoadingReferences: boolean;
  isLoadingReferencesNextPage: boolean;
  lastCatalogRefreshLabel: string;
  catalogCards: PackageCardDto[];
  catalogNextCursor: number | null;
  catalogHasMore: boolean;
  catalogPageSize: number;
  selectedPackageDetail?: PackageDetailDto;
  referenceRowsData: ReferenceRow[];
  referenceNextCursor: number | null;
  referenceHasMore: boolean;
  referencePageSize: number;
  catalogError: string | null;
  referenceError: string | null;
  profileError: string | null;
  downloadError: string | null;
  cacheError: string | null;
  settingsError: string | null;
  desktopError: string | null;
}

export interface SyncCatalogInput {
  force?: boolean;
}

export interface SearchPackagesInput {
  query: string;
  visibleStatuses: EffectiveStatus[];
  sortMode: BrowseSortMode;
  cursor?: number | null;
  pageSize?: number;
}

export interface SetReferenceStateInput {
  packageId: string;
  versionId: string;
  referenceState: ReferenceState;
}
