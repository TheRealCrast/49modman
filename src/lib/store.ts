import { derived, get, writable } from "svelte/store";
import {
  clearCache,
  clearCacheUnreferenced,
  getCacheSummary,
  openCacheFolder,
  previewClearCacheUnreferenced,
  queueInstallToCache
} from "./api/cache";
import { getCatalogSummary, getPackageDetail, searchPackages, syncCatalog } from "./api/catalog";
import { getVersionDependencies, warmDependencyIndex } from "./api/dependencies";
import { listActiveDownloads } from "./api/downloads";
import {
  getLaunchRuntimeStatus as getLaunchRuntimeStatusApi,
  getMemoryDiagnostics as getMemoryDiagnosticsApi,
  launchProfile as launchProfileApi,
  launchVanilla as launchVanillaApi,
  listProtonRuntimes as listProtonRuntimesApi,
  repairActivation as repairActivationApi,
  setPreferredProtonRuntime as setPreferredProtonRuntimeApi,
  trimResourceSaverMemory as trimResourceSaverMemoryApi
} from "./api/launch";
import {
  createProfile as createProfileApi,
  deleteProfile as deleteProfileApi,
  exportProfilePack as exportProfilePackApi,
  getActiveProfile as getActiveProfileApi,
  getUninstallDependants as getUninstallDependantsApi,
  importProfileModZip as importProfileModZipApi,
  importProfilePackFromPath as importProfilePackFromPathApi,
  previewImportProfilePack as previewImportProfilePackApi,
  getProfilesStorageSummary as getProfilesStorageSummaryApi,
  listProfiles as listProfilesApi,
  openActiveProfileFolder as openActiveProfileFolderApi,
  openProfilesFolder as openProfilesFolderApi,
  previewExportProfilePack as previewExportProfilePackApi,
  resetAllData as resetAllDataApi,
  setActiveProfile as setActiveProfileApi,
  setInstalledModEnabled as setInstalledModEnabledApi,
  uninstallInstalledMod as uninstallInstalledModApi,
  updateProfile as updateProfileApi
} from "./api/profiles";
import { listReferenceRows, setReferenceState as setReferenceStateApi } from "./api/reference";
import {
  getStorageLocations,
  getStorageMigrationStatus,
  getWarningPrefs,
  pickStorageFolder,
  startStorageMigration,
  setWarningPreference as setWarningPreferenceApi
} from "./api/settings";
import { openExternalUrl } from "./api/system";
import { seedActivities, seedPackages } from "./mock-data";
import { getRuntimeKind } from "./runtime";
import { compareVersionNumbers, resolveEffectiveStatus } from "./status";
import type {
  ActivityItem,
  AppState,
  AppView,
  BrowseSortMode,
  CacheSummaryDto,
  CreateProfileInput,
  DependencySummaryItemDto,
  DependencyModalState,
  DownloadJobDto,
  EffectiveStatus,
  InstallActionOptions,
  FocusedVersionState,
  ImportProfilePackPreviewModDto,
  ImportProfilePackPreviewResult,
  ImportProfileModZipPreviewResult,
  ImportProfileModZipResult,
  InstallRequest,
  LaunchMode,
  LaunchResult,
  ModPackage,
  ProfileDetailDto,
  ProfileInstalledModDto,
  ReferenceState,
  ResetProgressStep,
  StorageMigrationStatusDto
} from "./types";

const defaultVisibleStatuses: EffectiveStatus[] = ["verified", "green", "yellow", "orange"];
const defaultBrowseSortMode: BrowseSortMode = "mostDownloads";
const defaultCatalogPageSize = 40;
const defaultReferencePageSize = 50;
const downloadPollIntervalMs = 500;
const launchRuntimePollIntervalMs = 2500;
const memoryDiagnosticsPollIntervalMs = 2000;
const storageMigrationPollIntervalMs = 200;
const resourceSaverTransitionPollThreshold = 2;
const resourceSaverBlockNoticeCooldownMs = 4000;
let downloadPollHandle: number | null = null;
let launchRuntimePollHandle: number | null = null;
let memoryDiagnosticsPollHandle: number | null = null;
let storageMigrationPollHandle: number | null = null;
let isLoadingMemoryDiagnostics = false;
let isLoadingLaunchRuntimeStatus = false;
let consecutiveGameRunningPolls = 0;
let consecutiveGameStoppedPolls = 0;
let isResourceSaverTransitionInFlight = false;
let resourceSaverBlockNoticeLastAtMs = 0;
let focusedVersionClearHandle: number | null = null;
const busyPackageRefCounts = new Map<string, number>();
const activeInstallTaskPackageIds = new Map<string, string>();
let pendingWarningConfirmationResolver: ((confirmed: boolean) => void) | null = null;
let pendingUninstallDependantsConfirmationResolver: ((confirmed: boolean) => void) | null = null;
let pendingInstallWithoutDependenciesConfirmationResolver: ((confirmed: boolean) => void) | null =
  null;
let pendingImportModZipDecisionResolver: ((addToCache: boolean | null) => void) | null = null;

const initialState: AppState = {
  view: "browse",
  runtimeKind: getRuntimeKind(),
  browseSearchDraft: "",
  browseSearchSubmitted: "",
  browseSortMode: defaultBrowseSortMode,
  visibleStatuses: defaultVisibleStatuses,
  selectedPackageId: "bepinex-pack",
  selectedProfileId: "default",
  packages: seedPackages,
  profiles: [],
  activeProfile: undefined,
  downloads: [],
  cacheSummary: undefined,
  clearUnreferencedCacheModal: null,
  exportProfilePackModal: null,
  profilesStorageSummary: undefined,
  activeCacheTaskIds: [],
  busyPackageIds: [],
  activities: seedActivities,
  protonRuntimes: [],
  selectedProtonRuntimeId: null,
  isLoadingProtonRuntimes: false,
  isLaunching: false,
  launchingVariant: null,
  launchFeedback: null,
  warningPrefs: {
    red: true,
    broken: true,
    installWithoutDependencies: true,
    uninstallWithDependants: true,
    importProfilePack: true,
    conserveWhileGameRunning: false
  },
  storageLocations: undefined,
  storageMigration: null,
  isGameRunning: false,
  resourceSaverActive: false,
  resourceSaverLastView: null,
  modal: null,
  uninstallDependantsModal: null,
  installWithoutDependenciesModal: null,
  importProfilePackModal: null,
  importModZipModal: null,
  memoryDiagnosticsModal: null,
  resetProgress: null,
  dependencyModal: null,
  focusedVersion: null,
  referenceSearchDraft: "",
  referenceSearchSubmitted: "",
  isRefreshingCatalog: false,
  isBootstrapping: false,
  isCatalogOverlayVisible: false,
  catalogOverlayTitle: null,
  catalogOverlayMessage: null,
  catalogOverlayStep: null,
  isLoadingCatalogFirstPage: false,
  isLoadingCatalogNextPage: false,
  isLoadingPackageDetail: false,
  isLoadingProfiles: false,
  isLoadingDownloads: false,
  isLoadingCacheSummary: false,
  isLoadingProfilesStorageSummary: false,
  isLoadingReferences: false,
  isLoadingReferencesNextPage: false,
  lastCatalogRefreshLabel: "Cached mod list ready",
  catalogCards: [],
  catalogNextCursor: null,
  catalogHasMore: false,
  catalogPageSize: defaultCatalogPageSize,
  selectedPackageDetail: undefined,
  referenceRowsData: [],
  referenceNextCursor: null,
  referenceHasMore: false,
  referencePageSize: defaultReferencePageSize,
  catalogError: null,
  referenceError: null,
  profileError: null,
  downloadError: null,
  cacheError: null,
  settingsError: null,
  desktopError: null
};

export const appState = writable(initialState);

function findPackage(state: AppState, packageId: string): ModPackage | undefined {
  return state.packages.find((pkg) => pkg.id === packageId);
}

function appendActivity(state: AppState, item: ActivityItem): AppState {
  return {
    ...state,
    activities: [item, ...state.activities].slice(0, 6)
  };
}

function mapActiveProfile(state: AppState, activeProfile: ProfileDetailDto | undefined): AppState {
  return {
    ...state,
    activeProfile,
    selectedProfileId: activeProfile?.id ?? "default"
  };
}

function withActivity(title: string, detail: string, tone: ActivityItem["tone"]) {
  return {
    id: `activity-${Date.now()}`,
    title,
    detail,
    tone
  };
}

function describeUnknownError(error: unknown, fallback: string) {
  if (error instanceof Error && error.message.trim().length > 0) {
    return error.message;
  }

  if (typeof error === "string" && error.trim().length > 0) {
    return error;
  }

  if (error && typeof error === "object") {
    const record = error as Record<string, unknown>;
    const message = typeof record.message === "string" ? record.message.trim() : "";
    const code = typeof record.code === "string" ? record.code.trim() : "";
    const detail = typeof record.detail === "string" ? record.detail.trim() : "";

    if (message && detail) {
      return `${code ? `${code}: ` : ""}${message} (${detail})`;
    }
    if (message) {
      return `${code ? `${code}: ` : ""}${message}`;
    }
    if (detail) {
      return detail;
    }
  }

  return fallback;
}

function resolveLaunchMode(profile: ProfileDetailDto | undefined): LaunchMode {
  return profile?.launchModeDefault === "direct" ? "direct" : "steam";
}

function shouldOfferRepairForCode(code: string): boolean {
  return code === "ACTIVATION_FAILED" || code === "VANILLA_CLEANUP_INCOMPLETE";
}

function toFileUrl(path: string): string {
  if (path.startsWith("file://")) {
    return path;
  }

  const normalized = path.replace(/\\/g, "/");
  const prefix = normalized.startsWith("/") ? "file://" : "file:///";
  return `${prefix}${encodeURI(normalized)}`;
}

function mergeMockReferenceState(
  packages: ModPackage[],
  packageId: string,
  versionId: string,
  referenceState: ReferenceState
): ModPackage[] {
  return packages.map((pkg) =>
    pkg.id !== packageId
      ? pkg
      : {
          ...pkg,
          versions: pkg.versions.map((version) =>
            version.id !== versionId
              ? version
              : {
                  ...version,
                  overrideReferenceState: referenceState,
                  overrideReferenceNote:
                    referenceState === "neutral"
                      ? undefined
                      : referenceState === "verified"
                        ? "Locally marked verified from the prototype reference editor."
                        : "Locally marked broken from the prototype reference editor."
                }
          )
        }
  );
}

function waitForNextPaint() {
  return new Promise<void>((resolve) => {
    requestAnimationFrame(() => resolve());
  });
}

function isStorageMigrationBlocking(state = get(appState)) {
  return state.storageMigration?.isActive ?? false;
}

function stopDownloadPolling() {
  if (downloadPollHandle !== null) {
    window.clearInterval(downloadPollHandle);
    downloadPollHandle = null;
  }
}

function stopStorageMigrationPolling() {
  if (storageMigrationPollHandle !== null) {
    window.clearInterval(storageMigrationPollHandle);
    storageMigrationPollHandle = null;
  }
}

async function loadStorageMigrationStatus() {
  try {
    const status = await getStorageMigrationStatus();
    appState.update((state) => ({
      ...state,
      storageMigration: status
    }));

    if (!status.isActive) {
      stopStorageMigrationPolling();
      if (status.phase === "failed") {
        appState.update((state) => ({
          ...state,
          settingsError: status.error ?? status.message,
          desktopError:
            state.runtimeKind === "tauri"
              ? status.error ?? status.message
              : state.desktopError
        }));
      } else if (status.phase === "idle") {
        appState.update((state) => ({
          ...state,
          storageMigration: null
        }));
      }
      syncLaunchRuntimePollingForState(get(appState));
      syncDownloadPollingForState(get(appState));
    }
  } catch (error) {
    stopStorageMigrationPolling();
    appState.update((state) => ({
      ...state,
      storageMigration: null,
      settingsError:
        error instanceof Error
          ? error.message
          : "Failed to load storage migration progress.",
      desktopError:
        state.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to load desktop storage migration progress."
          : state.desktopError
    }));
    syncLaunchRuntimePollingForState(get(appState));
    syncDownloadPollingForState(get(appState));
  }
}

function startStorageMigrationPolling() {
  if (storageMigrationPollHandle !== null) {
    return;
  }

  void loadStorageMigrationStatus();
  storageMigrationPollHandle = window.setInterval(() => {
    void loadStorageMigrationStatus();
  }, storageMigrationPollIntervalMs);
}

function resetResourceSaverPollCounters() {
  consecutiveGameRunningPolls = 0;
  consecutiveGameStoppedPolls = 0;
}

function isHeavyWorkBlocked(state = get(appState)) {
  if (isStorageMigrationBlocking(state)) {
    return true;
  }
  return state.warningPrefs.conserveWhileGameRunning && state.resourceSaverActive;
}

function notifyHeavyWorkBlocked(actionLabel: string) {
  if (isStorageMigrationBlocking()) {
    appState.update((state) =>
      appendActivity(
        state,
        withActivity(
          "Storage migration in progress",
          `${actionLabel} is paused while storage data is being moved.`,
          "warning"
        )
      )
    );
    return;
  }

  const now = Date.now();
  if (now - resourceSaverBlockNoticeLastAtMs < resourceSaverBlockNoticeCooldownMs) {
    return;
  }
  resourceSaverBlockNoticeLastAtMs = now;

  appState.update((state) =>
    appendActivity(
      state,
      withActivity(
        "Resource saver active",
        `${actionLabel} is paused while Lethal Company is running.`,
        "warning"
      )
    )
  );
}

async function enterResourceSaverMode() {
  const snapshot = get(appState);
  if (!snapshot.warningPrefs.conserveWhileGameRunning || snapshot.resourceSaverActive) {
    return;
  }
  if (isResourceSaverTransitionInFlight) {
    return;
  }
  isResourceSaverTransitionInFlight = true;

  stopDownloadPolling();
  clearFocusedVersionTimer();

  appState.update((state) => ({
    ...state,
    resourceSaverActive: true,
    resourceSaverLastView: state.view !== "overview" ? state.view : state.resourceSaverLastView,
    view: "overview",
    catalogCards: [],
    catalogNextCursor: null,
    catalogHasMore: false,
    selectedPackageDetail: undefined,
    dependencyModal: null,
    focusedVersion: null,
    isLoadingCatalogFirstPage: false,
    isLoadingCatalogNextPage: false,
    isLoadingPackageDetail: false,
    referenceRowsData: [],
    referenceNextCursor: null,
    referenceHasMore: false,
    isLoadingReferences: false,
    isLoadingReferencesNextPage: false
  }));

  try {
    await trimResourceSaverMemoryApi();
    appState.update((state) =>
      appendActivity(
        {
          ...state,
          desktopError: null
        },
        withActivity(
          "Resource saver enabled",
          "Paused heavy UI data and released dependency index memory while the game is running.",
          "neutral"
        )
      )
    );
  } catch (error) {
    appState.update((state) => ({
      ...state,
      desktopError:
        state.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to trim desktop runtime memory for resource saver mode."
          : state.desktopError
    }));
  } finally {
    syncDownloadPollingForState(get(appState));
    isResourceSaverTransitionInFlight = false;
  }
}

async function warmDependencyIndexAfterResourceSaver() {
  try {
    await warmDependencyIndex();
  } catch (error) {
    appState.update((state) => ({
      ...state,
      desktopError:
        state.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to warm dependency index after resource saver mode."
          : state.desktopError
    }));
  }
}

async function exitResourceSaverMode() {
  const snapshot = get(appState);
  if (!snapshot.resourceSaverActive) {
    return;
  }
  if (isResourceSaverTransitionInFlight) {
    return;
  }
  isResourceSaverTransitionInFlight = true;

  const restoreView = snapshot.resourceSaverLastView ?? "overview";
  appState.update((state) => ({
    ...state,
    resourceSaverActive: false,
    resourceSaverLastView: null,
    view: restoreView
  }));

  try {
    const refreshedState = get(appState);
    await Promise.all([
      loadActiveDownloads(),
      loadCacheSummary(),
      refreshActiveProfileState(),
      refreshedState.view === "settings" ? loadProfilesStorageSummary() : Promise.resolve(),
      loadCatalogFirstPage({ showLoading: false, waitForSelectedPackageDetail: false }),
      warmDependencyIndexAfterResourceSaver()
    ]);

    appState.update((state) =>
      appendActivity(
        state,
        withActivity(
          "Resource saver disabled",
          "Restored normal background updates and rewarmed dependency cache after game exit.",
          "positive"
        )
      )
    );
  } catch (error) {
    appState.update((state) => ({
      ...state,
      desktopError:
        state.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to restore desktop state after resource saver mode."
          : state.desktopError
    }));
  } finally {
    syncDownloadPollingForState(get(appState));
    isResourceSaverTransitionInFlight = false;
  }
}

function shouldConserveResources(state: AppState) {
  return (
    state.warningPrefs.conserveWhileGameRunning &&
    (state.resourceSaverActive || state.isGameRunning)
  );
}

function syncDownloadPollingForState(state = get(appState)) {
  if (isStorageMigrationBlocking(state)) {
    stopDownloadPolling();
    return;
  }

  if (shouldConserveResources(state)) {
    stopDownloadPolling();
    return;
  }

  if (state.activeCacheTaskIds.length > 0) {
    startDownloadPolling();
  } else {
    stopDownloadPolling();
  }
}

function stopLaunchRuntimePolling() {
  if (launchRuntimePollHandle !== null) {
    window.clearInterval(launchRuntimePollHandle);
    launchRuntimePollHandle = null;
  }
  resetResourceSaverPollCounters();
}

function startLaunchRuntimePolling() {
  if (launchRuntimePollHandle !== null) {
    return;
  }

  resetResourceSaverPollCounters();
  void loadLaunchRuntimeStatus();
  launchRuntimePollHandle = window.setInterval(() => {
    void loadLaunchRuntimeStatus();
  }, launchRuntimePollIntervalMs);
}

function syncLaunchRuntimePollingForState(state = get(appState)) {
  if (isStorageMigrationBlocking(state)) {
    stopLaunchRuntimePolling();
    return;
  }

  if (state.warningPrefs.conserveWhileGameRunning) {
    startLaunchRuntimePolling();
    return;
  }

  stopLaunchRuntimePolling();
  if (state.resourceSaverActive) {
    void exitResourceSaverMode();
    return;
  }

  if (state.isGameRunning) {
    appState.update((current) => ({
      ...current,
      isGameRunning: false
    }));
  }
}

function stopMemoryDiagnosticsPolling() {
  if (memoryDiagnosticsPollHandle !== null) {
    window.clearInterval(memoryDiagnosticsPollHandle);
    memoryDiagnosticsPollHandle = null;
  }
}

function startMemoryDiagnosticsPolling() {
  if (memoryDiagnosticsPollHandle !== null) {
    return;
  }

  memoryDiagnosticsPollHandle = window.setInterval(() => {
    void loadMemoryDiagnosticsSnapshot();
  }, memoryDiagnosticsPollIntervalMs);
}

async function loadMemoryDiagnosticsSnapshot(options: { showLoading?: boolean } = {}) {
  const { showLoading = false } = options;
  const snapshot = get(appState).memoryDiagnosticsModal;
  if (!snapshot || isLoadingMemoryDiagnostics) {
    return;
  }

  isLoadingMemoryDiagnostics = true;
  if (showLoading) {
    appState.update((state) => ({
      ...state,
      memoryDiagnosticsModal: state.memoryDiagnosticsModal
        ? {
            ...state.memoryDiagnosticsModal,
            isLoading: true,
            error: null
          }
        : state.memoryDiagnosticsModal
    }));
  }

  try {
    const diagnostics = await getMemoryDiagnosticsApi();

    appState.update((state) => ({
      ...state,
      memoryDiagnosticsModal: state.memoryDiagnosticsModal
        ? {
            isLoading: false,
            data: diagnostics,
            error: null
          }
        : state.memoryDiagnosticsModal
    }));
  } catch (error) {
    const message = describeUnknownError(error, "Failed to load process memory diagnostics.");

    appState.update((state) => ({
      ...state,
      memoryDiagnosticsModal: state.memoryDiagnosticsModal
        ? {
            ...state.memoryDiagnosticsModal,
            isLoading: false,
            error: message
          }
        : state.memoryDiagnosticsModal
    }));
  } finally {
    isLoadingMemoryDiagnostics = false;
  }
}

function clearFocusedVersionTimer() {
  if (focusedVersionClearHandle !== null) {
    window.clearTimeout(focusedVersionClearHandle);
    focusedVersionClearHandle = null;
  }
}

function syncBusyPackageState() {
  appState.update((state) => ({
    ...state,
    busyPackageIds: [...busyPackageRefCounts.keys()]
  }));
}

function acquirePackageBusy(packageId: string) {
  const current = busyPackageRefCounts.get(packageId) ?? 0;
  busyPackageRefCounts.set(packageId, current + 1);
  syncBusyPackageState();
}

function releasePackageBusy(packageId: string) {
  const current = busyPackageRefCounts.get(packageId);
  if (!current) {
    return;
  }

  if (current <= 1) {
    busyPackageRefCounts.delete(packageId);
  } else {
    busyPackageRefCounts.set(packageId, current - 1);
  }

  syncBusyPackageState();
}

function clearBusyPackages() {
  busyPackageRefCounts.clear();
  activeInstallTaskPackageIds.clear();
  syncBusyPackageState();
}

function scheduleFocusedVersionClear(focusedVersion: FocusedVersionState) {
  clearFocusedVersionTimer();
  focusedVersionClearHandle = window.setTimeout(() => {
    appState.update((state) =>
      state.focusedVersion?.highlightToken === focusedVersion.highlightToken &&
      state.focusedVersion.packageId === focusedVersion.packageId &&
      state.focusedVersion.versionId === focusedVersion.versionId
        ? {
            ...state,
            focusedVersion: null
          }
        : state
    );
    focusedVersionClearHandle = null;
  }, 2000);
}

function ensureStatusVisible(status: EffectiveStatus) {
  appState.update((state) =>
    state.visibleStatuses.includes(status)
      ? state
      : {
          ...state,
          visibleStatuses: [...state.visibleStatuses, status]
        }
  );
}

async function navigateToPackageVersionInBrowse(packageId: string, versionId: string) {
  const highlightToken = Date.now();
  const focusedVersion: FocusedVersionState = {
    packageId,
    versionId,
    highlightToken
  };

  appState.update((state) => ({
    ...state,
    view: "browse",
    selectedPackageId: packageId,
    selectedPackageDetail:
      state.selectedPackageDetail?.id === packageId ? state.selectedPackageDetail : undefined,
    dependencyModal: null,
    focusedVersion
  }));

  scheduleFocusedVersionClear(focusedVersion);
  await loadSelectedPackageDetail(packageId);

  const state = get(appState);
  if (state.selectedPackageDetail?.id !== packageId) {
    return;
  }

  const version = state.selectedPackageDetail.versions.find((entry) => entry.id === versionId);
  if (!version) {
    return;
  }

  ensureStatusVisible(resolveEffectiveStatus(version));
}

function startDownloadPolling() {
  if (downloadPollHandle !== null) {
    return;
  }

  downloadPollHandle = window.setInterval(() => {
    void loadActiveDownloads();
  }, downloadPollIntervalMs);
}

function appendDownloadActivity(downloads: DownloadJobDto[], taskIds: string[]) {
  if (taskIds.length !== 0) {
    return;
  }

  const latest = downloads[0];
  if (!latest) {
    return;
  }

  appState.update((state) =>
    appendActivity(
      state,
      withActivity(
        latest.status === "failed"
          ? "Install failed"
          : latest.cacheHit
            ? "Installed from cache"
            : "Cache updated",
        latest.status === "failed"
          ? latest.errorMessage ?? `Failed to install ${latest.packageName} ${latest.versionLabel}.`
          : latest.cacheHit
            ? `${latest.packageName} ${latest.versionLabel} was installed from the shared cache.`
            : `${latest.packageName} ${latest.versionLabel} was downloaded and installed.`,
        latest.status === "failed" ? "warning" : "positive"
      )
    )
  );
}

async function loadSelectedPackageDetail(packageId = get(appState).selectedPackageId) {
  if (isHeavyWorkBlocked()) {
    appState.update((current) => ({
      ...current,
      isLoadingPackageDetail: false
    }));
    return;
  }

  if (!packageId) {
    appState.update((current) => ({
      ...current,
      selectedPackageDetail: undefined,
      isLoadingPackageDetail: false
    }));
    return;
  }

  appState.update((current) => ({
    ...current,
    isLoadingPackageDetail: true
  }));

  try {
    const detail = await getPackageDetail(packageId);

    appState.update((current) => ({
      ...current,
      selectedPackageDetail: detail ?? undefined,
      isLoadingPackageDetail: false,
      catalogError: detail ? current.catalogError : "Selected package is no longer available.",
      desktopError: detail ? current.desktopError : current.desktopError
    }));
  } catch (error) {
    appState.update((current) => ({
      ...current,
      isLoadingPackageDetail: false,
      catalogError: error instanceof Error ? error.message : "Failed to load package details.",
      desktopError:
        current.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to load package details from the desktop backend."
          : current.desktopError
    }));
  }
}

async function loadProfilesState() {
  appState.update((current) => ({
    ...current,
    isLoadingProfiles: true,
    profileError: null
  }));

  try {
    const [profiles, activeProfile] = await Promise.all([listProfilesApi(), getActiveProfileApi()]);

    appState.update((current) => ({
      ...mapActiveProfile(current, activeProfile ?? undefined),
      profiles,
      isLoadingProfiles: false,
      profileError: null,
      desktopError: null
    }));
  } catch (error) {
    appState.update((current) => ({
      ...current,
      isLoadingProfiles: false,
      profileError: error instanceof Error ? error.message : "Failed to load profiles.",
      desktopError:
        current.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to load desktop profiles."
          : current.desktopError
    }));
  }
}

async function refreshActiveProfileState() {
  try {
    const activeProfile = await getActiveProfileApi();
    appState.update((current) => ({
      ...mapActiveProfile(current, activeProfile ?? undefined),
      profileError: null,
      desktopError: null
    }));
  } catch (error) {
    appState.update((current) => ({
      ...current,
      profileError: error instanceof Error ? error.message : "Failed to refresh the active profile.",
      desktopError:
        current.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to refresh the active desktop profile."
          : current.desktopError
    }));
  }
}

async function loadSettingsState() {
  const [warningPrefs, storageLocations, storageMigration] = await Promise.all([
    getWarningPrefs(),
    getStorageLocations(),
    getStorageMigrationStatus()
  ]);
  appState.update((state) => ({
    ...state,
    warningPrefs,
    storageLocations,
    storageMigration: storageMigration.isActive ? storageMigration : null,
    settingsError: null,
    desktopError: null
  }));
  if (storageMigration.isActive) {
    startStorageMigrationPolling();
  } else {
    stopStorageMigrationPolling();
  }
  syncLaunchRuntimePollingForState(get(appState));
  syncDownloadPollingForState(get(appState));
}

async function loadLaunchRuntimeStatus() {
  const snapshot = get(appState);
  if (
    isStorageMigrationBlocking(snapshot) ||
    !snapshot.warningPrefs.conserveWhileGameRunning ||
    isLoadingLaunchRuntimeStatus
  ) {
    return;
  }

  isLoadingLaunchRuntimeStatus = true;
  try {
    const status = await getLaunchRuntimeStatusApi();
    appState.update((state) => ({
      ...state,
      isGameRunning: status.isGameRunning
    }));

    if (status.isGameRunning) {
      consecutiveGameRunningPolls += 1;
      consecutiveGameStoppedPolls = 0;
      if (consecutiveGameRunningPolls >= resourceSaverTransitionPollThreshold) {
        void enterResourceSaverMode();
      }
    } else {
      consecutiveGameStoppedPolls += 1;
      consecutiveGameRunningPolls = 0;
      if (consecutiveGameStoppedPolls >= resourceSaverTransitionPollThreshold) {
        void exitResourceSaverMode();
      }
    }
  } catch {
    // Keep this silent to avoid poll-loop error spam in the UI.
  } finally {
    isLoadingLaunchRuntimeStatus = false;
    syncDownloadPollingForState(get(appState));
  }
}

async function loadProtonRuntimesState() {
  appState.update((state) => ({
    ...state,
    isLoadingProtonRuntimes: true
  }));

  try {
    const protonRuntimes = await listProtonRuntimesApi();
    appState.update((state) => {
      const validIds = new Set(protonRuntimes.filter((entry) => entry.isValid).map((entry) => entry.id));
      const selectedProtonRuntimeId =
        state.selectedProtonRuntimeId && validIds.has(state.selectedProtonRuntimeId)
          ? state.selectedProtonRuntimeId
          : protonRuntimes.find((entry) => entry.isValid)?.id ?? null;

      return {
        ...state,
        protonRuntimes,
        selectedProtonRuntimeId,
        isLoadingProtonRuntimes: false,
        desktopError: null
      };
    });
  } catch (error) {
    appState.update((state) => ({
      ...state,
      isLoadingProtonRuntimes: false,
      desktopError:
        state.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to detect Proton runtimes from the desktop backend."
          : state.desktopError
    }));
  }
}

async function loadProfilesStorageSummary() {
  appState.update((current) => ({
    ...current,
    isLoadingProfilesStorageSummary: true
  }));

  try {
    const profilesStorageSummary = await getProfilesStorageSummaryApi();
    appState.update((current) => ({
      ...current,
      profilesStorageSummary,
      isLoadingProfilesStorageSummary: false,
      settingsError: null,
      desktopError: null
    }));
  } catch (error) {
    appState.update((current) => ({
      ...current,
      isLoadingProfilesStorageSummary: false,
      settingsError:
        error instanceof Error ? error.message : "Failed to load profile storage summary.",
      desktopError:
        current.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to load desktop profile storage summary."
          : current.desktopError
    }));
  }
}

async function loadCacheSummary() {
  appState.update((current) => ({
    ...current,
    isLoadingCacheSummary: true,
    cacheError: null
  }));

  try {
    const cacheSummary = await getCacheSummary();
    appState.update((current) => ({
      ...current,
      cacheSummary,
      isLoadingCacheSummary: false,
      cacheError: null,
      desktopError: null
    }));
  } catch (error) {
    appState.update((current) => ({
      ...current,
      isLoadingCacheSummary: false,
      cacheError: error instanceof Error ? error.message : "Failed to load the cache summary.",
      desktopError:
        current.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to load desktop cache data."
          : current.desktopError
    }));
  }
}

async function loadActiveDownloads() {
  const previous = get(appState);
  appState.update((current) => ({
    ...current,
    isLoadingDownloads: true
  }));

  try {
    const downloads = await listActiveDownloads();
    const activeTaskIds = [
      ...new Set(downloads.filter((entry) => entry.status !== "failed").map((entry) => entry.taskId))
    ];
    for (const [taskId, packageId] of activeInstallTaskPackageIds.entries()) {
      if (!activeTaskIds.includes(taskId)) {
        activeInstallTaskPackageIds.delete(taskId);
        releasePackageBusy(packageId);
      }
    }
    const hadActiveTasks = previous.activeCacheTaskIds.length > 0;

    appState.update((current) => ({
      ...current,
      downloads,
      activeCacheTaskIds: activeTaskIds,
      isLoadingDownloads: false,
      downloadError: null,
      desktopError: null
    }));

    syncDownloadPollingForState(get(appState));
    if (activeTaskIds.length === 0 && hadActiveTasks) {
      await Promise.all([loadCacheSummary(), refreshActiveProfileState(), loadProfilesStorageSummary()]);
      appendDownloadActivity(previous.downloads, activeTaskIds);
    }
  } catch (error) {
    stopDownloadPolling();
    appState.update((current) => ({
      ...current,
      isLoadingDownloads: false,
      downloadError: error instanceof Error ? error.message : "Failed to load active downloads.",
      desktopError:
        current.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to load active desktop downloads."
          : current.desktopError
    }));
  }
}

async function loadCatalogFirstPage(
  options: { showLoading?: boolean; waitForSelectedPackageDetail?: boolean } = {}
) {
  if (isHeavyWorkBlocked()) {
    appState.update((current) => ({
      ...current,
      isLoadingCatalogFirstPage: false
    }));
    return false;
  }

  const { showLoading = true, waitForSelectedPackageDetail = false } = options;
  const state = get(appState);

  if (showLoading) {
    appState.update((current) => ({
      ...current,
      isLoadingCatalogFirstPage: true,
      catalogError: null,
      desktopError: null
    }));
  }

  try {
    const result = await searchPackages({
      query: state.browseSearchSubmitted.trim(),
      visibleStatuses: state.visibleStatuses,
      sortMode: state.browseSortMode,
      cursor: 0,
      pageSize: state.catalogPageSize
    });

    const nextSelection = result.items.some((card) => card.id === state.selectedPackageId)
      ? state.selectedPackageId
      : result.items[0]?.id ?? "";

    appState.update((current) => ({
      ...current,
      catalogOverlayStep: current.isCatalogOverlayVisible ? "browse" : current.catalogOverlayStep,
      catalogOverlayMessage: current.isCatalogOverlayVisible
        ? "Loading the first page of cached results"
        : current.catalogOverlayMessage,
      catalogCards: result.items,
      catalogNextCursor: result.nextCursor,
      catalogHasMore: result.hasMore,
      catalogPageSize: result.pageSize,
      selectedPackageId: nextSelection,
      selectedPackageDetail:
        nextSelection && nextSelection === current.selectedPackageId
          ? current.selectedPackageDetail
          : undefined,
      isLoadingCatalogFirstPage: false,
      catalogError: null,
      desktopError: null
    }));

    if (nextSelection) {
      if (waitForSelectedPackageDetail) {
        await loadSelectedPackageDetail(nextSelection);
      } else {
        void loadSelectedPackageDetail(nextSelection);
      }
    }
    return true;
  } catch (error) {
    appState.update((current) => ({
      ...current,
      isLoadingCatalogFirstPage: false,
      catalogError: error instanceof Error ? error.message : "Failed to load the catalog.",
      desktopError:
        current.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to load the catalog from the desktop backend."
          : current.desktopError
    }));
    return false;
  }
}

async function loadCatalogNextPage() {
  if (isHeavyWorkBlocked()) {
    appState.update((current) => ({
      ...current,
      isLoadingCatalogNextPage: false
    }));
    return;
  }

  const state = get(appState);

  if (state.isLoadingCatalogNextPage || !state.catalogHasMore || state.catalogNextCursor === null) {
    return;
  }

  appState.update((current) => ({
    ...current,
    isLoadingCatalogNextPage: true
  }));

  try {
    const result = await searchPackages({
      query: state.browseSearchSubmitted.trim(),
      visibleStatuses: state.visibleStatuses,
      sortMode: state.browseSortMode,
      cursor: state.catalogNextCursor,
      pageSize: state.catalogPageSize
    });

    appState.update((current) => ({
      ...current,
      catalogCards: [...current.catalogCards, ...result.items],
      catalogNextCursor: result.nextCursor,
      catalogHasMore: result.hasMore,
      catalogPageSize: result.pageSize,
      isLoadingCatalogNextPage: false,
      catalogError: null
    }));
  } catch (error) {
    appState.update((current) => ({
      ...current,
      isLoadingCatalogNextPage: false,
      catalogError: error instanceof Error ? error.message : "Failed to load more mods."
    }));
  }
}

async function loadReferenceLibrary() {
  if (isHeavyWorkBlocked()) {
    appState.update((current) => ({
      ...current,
      isLoadingReferences: false
    }));
    return;
  }

  const state = get(appState);

  appState.update((current) => ({
    ...current,
    isLoadingReferences: true,
    referenceError: null
  }));

  try {
    const result = await listReferenceRows({
      query: state.referenceSearchSubmitted.trim(),
      cursor: 0,
      pageSize: state.referencePageSize
    });

    appState.update((current) => ({
      ...current,
      referenceRowsData: result.items,
      referenceNextCursor: result.nextCursor,
      referenceHasMore: result.hasMore,
      referencePageSize: result.pageSize,
      referenceError: null,
      isLoadingReferences: false
    }));
  } catch (error) {
    appState.update((current) => ({
      ...current,
      isLoadingReferences: false,
      referenceError: error instanceof Error ? error.message : "Failed to load the reference library."
    }));
  }
}

async function loadMoreReferenceLibrary() {
  if (isHeavyWorkBlocked()) {
    appState.update((current) => ({
      ...current,
      isLoadingReferencesNextPage: false
    }));
    return;
  }

  const state = get(appState);

  if (
    state.isLoadingReferences ||
    state.isLoadingReferencesNextPage ||
    !state.referenceHasMore ||
    state.referenceNextCursor === null
  ) {
    return;
  }

  appState.update((current) => ({
    ...current,
    isLoadingReferencesNextPage: true,
    referenceError: null
  }));

  try {
    const result = await listReferenceRows({
      query: state.referenceSearchSubmitted.trim(),
      cursor: state.referenceNextCursor,
      pageSize: state.referencePageSize
    });

    appState.update((current) => ({
      ...current,
      referenceRowsData: [...current.referenceRowsData, ...result.items],
      referenceNextCursor: result.nextCursor,
      referenceHasMore: result.hasMore,
      referencePageSize: result.pageSize,
      isLoadingReferencesNextPage: false
    }));
  } catch (error) {
    appState.update((current) => ({
      ...current,
      isLoadingReferencesNextPage: false,
      referenceError: error instanceof Error ? error.message : "Failed to load more reference rows."
    }));
  }
}

async function warmDependencyIndexForOverlay() {
  appState.update((state) => ({
    ...state,
    catalogOverlayStep: state.isCatalogOverlayVisible ? "dependencies" : state.catalogOverlayStep,
    catalogOverlayMessage: state.isCatalogOverlayVisible
      ? "Preparing dependency index from cached metadata"
      : state.catalogOverlayMessage
  }));

  await warmDependencyIndex();
}

async function checkForCatalogUpdatesInStartupOverlay() {
  appState.update((state) => ({
    ...state,
    isRefreshingCatalog: true,
    catalogError: null,
    desktopError: null,
    catalogOverlayStep: state.isCatalogOverlayVisible ? "dependencies" : state.catalogOverlayStep,
    catalogOverlayMessage: state.isCatalogOverlayVisible
      ? "Checking for cached catalog updates"
      : state.catalogOverlayMessage
  }));

  try {
    const result = await syncCatalog({ force: false });

    if (result.outcome === "synced") {
      appState.update((state) => ({
        ...state,
        catalogOverlayStep: state.isCatalogOverlayVisible ? "dependencies" : state.catalogOverlayStep,
        catalogOverlayMessage: state.isCatalogOverlayVisible
          ? "Catalog updated. Reloading Browse results"
          : state.catalogOverlayMessage
      }));

      const reloaded = await loadCatalogFirstPage({
        showLoading: false,
        waitForSelectedPackageDetail: true
      });
      if (!reloaded) {
        throw new Error("The catalog cache refreshed, but the first page could not be loaded.");
      }

      await warmDependencyIndexForOverlay();
    }

    const summary = await getCatalogSummary();

    appState.update((state) =>
      appendActivity(
        {
          ...state,
          lastCatalogRefreshLabel: result.outcome === "synced" ? result.message : summary.lastSyncLabel,
          desktopError: null
        },
        withActivity(
          result.outcome === "synced" ? "Catalog refreshed" : "Catalog already fresh",
          result.outcome === "synced"
            ? `${result.packageCount} packages and ${result.versionCount} versions are cached locally.`
            : "The cached Thunderstore metadata is still within the freshness window.",
          "neutral"
        )
      )
    );
  } finally {
    appState.update((state) => ({
      ...state,
      isRefreshingCatalog: false
    }));
  }
}

async function refreshCatalog(
  force: boolean,
  options: {
    blockingOverlay?: boolean;
    includeDependencyWarm?: boolean;
    showFirstPageLoading?: boolean;
    waitForSelectedPackageDetail?: boolean;
  } = {}
) {
  const {
    blockingOverlay = false,
    includeDependencyWarm = blockingOverlay,
    showFirstPageLoading = !blockingOverlay,
    waitForSelectedPackageDetail = blockingOverlay
  } = options;

  appState.update((state) => ({
    ...state,
    isRefreshingCatalog: true,
    catalogError: null,
    desktopError: null,
    lastCatalogRefreshLabel: force ? "Refreshing cached mod list..." : state.lastCatalogRefreshLabel,
    isCatalogOverlayVisible: blockingOverlay,
    catalogOverlayTitle: blockingOverlay
      ? force
        ? "Refreshing Thunderstore catalog"
        : "Retrieving Thunderstore catalog"
      : null,
    catalogOverlayMessage: blockingOverlay
      ? force
        ? "Contacting Thunderstore and updating the local cache"
        : "Building local cache for Browse"
      : null,
    catalogOverlayStep: blockingOverlay ? "network" : null
  }));

  if (blockingOverlay) {
    await waitForNextPaint();
  }

  try {
    const result = await syncCatalog({ force });
    const summary = await getCatalogSummary();
    appState.update((state) => ({
      ...state,
      catalogOverlayStep: state.isCatalogOverlayVisible ? "cache" : state.catalogOverlayStep,
      catalogOverlayMessage: state.isCatalogOverlayVisible
        ? "Local metadata updated. Loading Browse results"
        : state.catalogOverlayMessage
    }));
    const reloaded = await loadCatalogFirstPage({
      showLoading: showFirstPageLoading,
      waitForSelectedPackageDetail
    });

    if (!reloaded) {
      throw new Error("The catalog cache refreshed, but the first page could not be loaded.");
    }

    if (blockingOverlay && includeDependencyWarm) {
      await warmDependencyIndexForOverlay();
    }

    appState.update((state) =>
      appendActivity(
        {
          ...state,
          isRefreshingCatalog: false,
          isCatalogOverlayVisible: false,
          catalogOverlayTitle: null,
          catalogOverlayMessage: null,
          catalogOverlayStep: null,
          lastCatalogRefreshLabel: result.outcome === "synced" ? result.message : summary.lastSyncLabel,
          desktopError: null
        },
        withActivity(
          result.outcome === "synced" ? "Catalog refreshed" : "Catalog already fresh",
          result.outcome === "synced"
            ? `${result.packageCount} packages and ${result.versionCount} versions are cached locally.`
            : "The cached Thunderstore metadata is still within the freshness window.",
          "neutral"
        )
      )
    );
  } catch (error) {
    appState.update((state) => ({
      ...state,
      isRefreshingCatalog: false,
      isCatalogOverlayVisible: false,
      catalogOverlayTitle: null,
      catalogOverlayMessage: null,
      catalogOverlayStep: null,
      catalogError: error instanceof Error ? error.message : "Failed to refresh the cached mod list.",
      desktopError:
        state.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to refresh the cached mod list from the desktop backend."
          : state.desktopError
    }));
  }
}

function findInstalledModsForPackage(
  installedMods: ProfileInstalledModDto[] | undefined,
  packageId: string
) {
  return (installedMods ?? []).filter((entry) => entry.packageId === packageId);
}

type InstallFlowOptions = {
  includeDependencies?: boolean;
  promptWithoutDependencies?: boolean;
};

type DependencyInstallPlanStep = {
  request: InstallRequest;
  switchFromVersionIds: string[];
  kind: "install" | "switch";
};

type DependencyInstallPlan = {
  steps: DependencyInstallPlanStep[];
  skippedAlreadyInstalledCount: number;
  skippedLowerOrEqualCount: number;
};

function resolvePendingWarningConfirmation(confirmed: boolean) {
  const resolver = pendingWarningConfirmationResolver;
  pendingWarningConfirmationResolver = null;
  if (resolver) {
    resolver(confirmed);
  }
}

function resolvePendingUninstallDependantsConfirmation(confirmed: boolean) {
  const resolver = pendingUninstallDependantsConfirmationResolver;
  pendingUninstallDependantsConfirmationResolver = null;
  if (resolver) {
    resolver(confirmed);
  }
}

function resolvePendingInstallWithoutDependenciesConfirmation(confirmed: boolean) {
  const resolver = pendingInstallWithoutDependenciesConfirmationResolver;
  pendingInstallWithoutDependenciesConfirmationResolver = null;
  if (resolver) {
    resolver(confirmed);
  }
}

function resolvePendingImportModZipDecision(addToCache: boolean | null) {
  const resolver = pendingImportModZipDecisionResolver;
  pendingImportModZipDecisionResolver = null;
  if (resolver) {
    resolver(addToCache);
  }
}

async function confirmWarningForInstallIfNeeded(
  request: InstallRequest,
  switchFromVersionIds: string[] = []
) {
  const state = get(appState);
  const uniqueSwitchFromVersionIds = [...new Set(switchFromVersionIds)];
  const shouldWarnBroken = request.effectiveStatus === "broken" && state.warningPrefs.broken;
  const shouldWarnRed = request.effectiveStatus === "red" && state.warningPrefs.red;
  if (!shouldWarnBroken && !shouldWarnRed) {
    return true;
  }

  if (pendingWarningConfirmationResolver) {
    resolvePendingWarningConfirmation(false);
  }

  appState.update((current) => ({
    ...current,
    modal: {
      packageId: request.packageId,
      packageName: request.packageName,
      versionId: request.versionId,
      versionNumber: request.versionNumber,
      status: shouldWarnBroken ? "broken" : "red",
      referenceNote: request.referenceNote,
      switchFromVersionIds:
        uniqueSwitchFromVersionIds.length > 0 ? uniqueSwitchFromVersionIds : undefined
    }
  }));

  return new Promise<boolean>((resolve) => {
    pendingWarningConfirmationResolver = resolve;
  });
}

async function confirmInstallWithoutDependenciesIfNeeded(request: InstallRequest) {
  const state = get(appState);
  if (!state.warningPrefs.installWithoutDependencies) {
    return true;
  }

  if (pendingInstallWithoutDependenciesConfirmationResolver) {
    resolvePendingInstallWithoutDependenciesConfirmation(false);
  }

  appState.update((current) => ({
    ...current,
    installWithoutDependenciesModal: {
      packageName: request.packageName,
      versionNumber: request.versionNumber
    }
  }));

  return new Promise<boolean>((resolve) => {
    pendingInstallWithoutDependenciesConfirmationResolver = resolve;
  });
}

async function confirmImportModZipWithModal(preview: ImportProfileModZipPreviewResult) {
  if (pendingImportModZipDecisionResolver) {
    resolvePendingImportModZipDecision(null);
  }

  appState.update((current) => ({
    ...current,
    importModZipModal: {
      preview
    }
  }));

  return new Promise<boolean | null>((resolve) => {
    pendingImportModZipDecisionResolver = resolve;
  });
}

async function confirmUninstallWithDependantsIfNeeded(options: {
  profileId: string;
  packageId: string;
  packageName: string;
  versionIds: string[];
}) {
  const { profileId, packageId, packageName, versionIds } = options;
  const state = get(appState);
  if (!state.warningPrefs.uninstallWithDependants) {
    return true;
  }

  let dependants;
  try {
    dependants = await getUninstallDependantsApi({
      profileId,
      packageId,
      versionIds
    });
  } catch (error) {
    appState.update((current) => ({
      ...current,
      profileError:
        error instanceof Error
          ? error.message
          : "Failed to validate uninstall dependants in this profile.",
      desktopError:
        current.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to validate uninstall dependants in the desktop backend."
          : current.desktopError
    }));
    return false;
  }

  if (dependants.length === 0) {
    return true;
  }

  if (pendingUninstallDependantsConfirmationResolver) {
    resolvePendingUninstallDependantsConfirmation(false);
  }

  appState.update((current) => ({
    ...current,
    uninstallDependantsModal: {
      packageId,
      packageName,
      versionIds: [...new Set(versionIds)],
      dependants
    }
  }));

  return new Promise<boolean>((resolve) => {
    pendingUninstallDependantsConfirmationResolver = resolve;
  });
}

function buildDependencyInstallPlan(
  rootRequest: InstallRequest,
  installedMods: ProfileInstalledModDto[],
  dependencies: DependencySummaryItemDto[]
): DependencyInstallPlan {
  const steps: DependencyInstallPlanStep[] = [];
  let skippedAlreadyInstalledCount = 0;
  let skippedLowerOrEqualCount = 0;
  const installedByPackage = new Map<string, ProfileInstalledModDto[]>();

  for (const entry of installedMods) {
    const existing = installedByPackage.get(entry.packageId) ?? [];
    installedByPackage.set(entry.packageId, [...existing, entry]);
  }

  for (const dependency of dependencies) {
    if (dependency.packageId === rootRequest.packageId) {
      continue;
    }

    const installedForPackage = installedByPackage.get(dependency.packageId) ?? [];

    if (installedForPackage.some((entry) => entry.versionId === dependency.versionId)) {
      skippedAlreadyInstalledCount += 1;
      continue;
    }

    const dependencyRequest: InstallRequest = {
      packageId: dependency.packageId,
      packageName: dependency.packageName,
      versionId: dependency.versionId,
      versionNumber: dependency.versionNumber,
      effectiveStatus: dependency.effectiveStatus,
      referenceNote: dependency.referenceNote
    };

    if (installedForPackage.length === 0) {
      steps.push({
        request: dependencyRequest,
        switchFromVersionIds: [],
        kind: "install"
      });
      installedByPackage.set(dependency.packageId, [
        {
          packageId: dependency.packageId,
          packageName: dependency.packageName,
          versionId: dependency.versionId,
          versionNumber: dependency.versionNumber,
          enabled: true,
          sourceKind: "thunderstore",
          installDir: "",
          installedAt: ""
        }
      ]);
      continue;
    }

    const highestInstalled = installedForPackage.reduce((best, current) =>
      compareVersionNumbers(current.versionNumber, best.versionNumber) > 0 ? current : best
    );
    if (compareVersionNumbers(dependency.versionNumber, highestInstalled.versionNumber) <= 0) {
      skippedLowerOrEqualCount += 1;
      continue;
    }

    steps.push({
      request: dependencyRequest,
      switchFromVersionIds: installedForPackage.map((entry) => entry.versionId),
      kind: "switch"
    });
    installedByPackage.set(dependency.packageId, [
      {
        packageId: dependency.packageId,
        packageName: dependency.packageName,
        versionId: dependency.versionId,
        versionNumber: dependency.versionNumber,
        enabled: true,
        sourceKind: "thunderstore",
        installDir: "",
        installedAt: ""
      }
    ]);
  }

  return {
    steps,
    skippedAlreadyInstalledCount,
    skippedLowerOrEqualCount
  };
}

async function performInstallOperation(request: InstallRequest, switchFromVersionIds: string[] = []) {
  const confirmed = await confirmWarningForInstallIfNeeded(request, switchFromVersionIds);
  if (!confirmed) {
    return false;
  }

  if (switchFromVersionIds.length > 0) {
    return switchInstalledPackageVersionInternal(request, switchFromVersionIds);
  }

  return queueVersionForCache(request);
}

async function queueDependencyInstallsForRequest(rootRequest: InstallRequest) {
  try {
    const resolvedDependencies = await getVersionDependencies({
      packageId: rootRequest.packageId,
      versionId: rootRequest.versionId
    });
    const dependencyItems = [
      ...resolvedDependencies.summary.direct,
      ...resolvedDependencies.summary.transitive
    ];
    const installedMods = get(appState).activeProfile?.installedMods ?? [];
    const plan = buildDependencyInstallPlan(rootRequest, installedMods, dependencyItems);

    if (plan.steps.length === 0) {
      if (resolvedDependencies.summary.unresolved.length > 0) {
        appState.update((state) =>
          appendActivity(
            state,
            withActivity(
              "Dependency install skipped",
              `${resolvedDependencies.summary.unresolved.length} dependenc${resolvedDependencies.summary.unresolved.length === 1 ? "y is" : "ies are"} unresolved in the cached catalog.`,
              "warning"
            )
          )
        );
      }
      return;
    }

    let installedCount = 0;
    let switchedCount = 0;

    for (const step of plan.steps) {
      const started = await performInstallOperation(step.request, step.switchFromVersionIds);
      if (!started) {
        break;
      }

      if (step.kind === "switch") {
        switchedCount += 1;
      } else {
        installedCount += 1;
      }
    }

    const skippedCount = plan.skippedAlreadyInstalledCount + plan.skippedLowerOrEqualCount;
    const detailParts: string[] = [];
    if (installedCount > 0) {
      detailParts.push(
        `${installedCount} dependenc${installedCount === 1 ? "y" : "ies"} queued for install`
      );
    }
    if (switchedCount > 0) {
      detailParts.push(
        `${switchedCount} dependenc${switchedCount === 1 ? "y" : "ies"} queued for version switch`
      );
    }
    if (skippedCount > 0) {
      detailParts.push(`${skippedCount} skipped`);
    }
    const detail =
      detailParts.length > 0 ? detailParts.join(" · ") : "No dependency installs were queued.";

    appState.update((state) =>
      appendActivity(
        state,
        withActivity(
          "Dependency actions queued",
          detail,
          resolvedDependencies.summary.unresolved.length > 0 ? "warning" : "neutral"
        )
      )
    );
  } catch (error) {
    const fallbackMessage = "Failed to resolve dependencies for install.";
    const errorMessage = error instanceof Error ? `${fallbackMessage} ${error.message}` : fallbackMessage;
    appState.update((state) =>
      appendActivity(
        {
          ...state,
          desktopError: state.runtimeKind === "tauri" ? errorMessage : state.desktopError
        },
        withActivity("Dependency install skipped", errorMessage, "warning")
      )
    );
  }
}

async function requestInstallWithOptionalSwitch(
  request: InstallRequest,
  switchFromVersionIds: string[] = [],
  options: InstallFlowOptions = {}
) {
  const { includeDependencies = true, promptWithoutDependencies = false } = options;

  if (!includeDependencies && promptWithoutDependencies) {
    const confirmed = await confirmInstallWithoutDependenciesIfNeeded(request);
    if (!confirmed) {
      return;
    }
  }

  const started = await performInstallOperation(request, switchFromVersionIds);
  if (!started) {
    return;
  }

  if (includeDependencies) {
    await queueDependencyInstallsForRequest(request);
  }
}

async function uninstallInstalledVersionsForActiveProfile(options: {
  packageId: string;
  packageName: string;
  versionIds: string[];
  title: string;
  detail: string;
  tone: ActivityItem["tone"];
  managePackageLock?: boolean;
  warnOnDependants?: boolean;
}) {
  const {
    packageId,
    packageName,
    versionIds,
    title,
    detail,
    tone,
    managePackageLock = true,
    warnOnDependants = true
  } = options;
  const uniqueVersionIds = [...new Set(versionIds)];

  if (uniqueVersionIds.length === 0) {
    return true;
  }

  if (managePackageLock) {
    acquirePackageBusy(packageId);
  }

  const state = get(appState);
  const activeProfile = state.activeProfile;

  if (!activeProfile) {
    if (managePackageLock) {
      releasePackageBusy(packageId);
    }
    return false;
  }

  try {
    if (warnOnDependants) {
      const confirmed = await confirmUninstallWithDependantsIfNeeded({
        profileId: activeProfile.id,
        packageId,
        packageName,
        versionIds: uniqueVersionIds
      });
      if (!confirmed) {
        return false;
      }
    }

    let nextActiveProfile = activeProfile;

    for (const versionId of uniqueVersionIds) {
      nextActiveProfile = await uninstallInstalledModApi({
        profileId: activeProfile.id,
        packageId,
        versionId
      });
    }

    const [profiles, profilesStorageSummary] = await Promise.all([
      listProfilesApi(),
      getProfilesStorageSummaryApi()
    ]);

    appState.update((current) =>
      appendActivity(
        {
          ...mapActiveProfile(current, nextActiveProfile),
          profiles,
          profilesStorageSummary,
          profileError: null,
          desktopError: null
        },
        withActivity(title, detail, tone)
      )
    );

    return true;
  } catch (error) {
    appState.update((current) => ({
      ...current,
      profileError: error instanceof Error ? error.message : "Failed to uninstall the mod.",
      desktopError:
        current.runtimeKind === "tauri"
          ? error instanceof Error
            ? error.message
            : "Failed to uninstall the mod in the desktop backend."
          : current.desktopError
    }));
    return false;
  } finally {
    if (managePackageLock) {
      releasePackageBusy(packageId);
    }
  }
}

function dependencyModalMatches(
  modal: DependencyModalState | null,
  packageId: string,
  versionId: string
) {
  return modal?.packageId === packageId && modal.versionId === versionId;
}

async function queueVersionForCache(
  request: InstallRequest,
  options: { lockAlreadyHeld?: boolean } = {}
) {
  const { lockAlreadyHeld = false } = options;
  if (!lockAlreadyHeld) {
    acquirePackageBusy(request.packageId);
  }

  try {
    const result = await queueInstallToCache({
      packageId: request.packageId,
      versionId: request.versionId
    });
    activeInstallTaskPackageIds.set(result.taskId, request.packageId);

    appState.update((current) =>
      appendActivity(
        {
          ...current,
          modal: null,
          downloadError: null,
          cacheError: null,
          desktopError: null,
          activeCacheTaskIds: current.activeCacheTaskIds.includes(result.taskId)
            ? current.activeCacheTaskIds
            : [result.taskId, ...current.activeCacheTaskIds]
        },
        withActivity(
          "Installing mod",
          `Preparing ${request.packageName} ${request.versionNumber} from cache into the active profile.`,
          "neutral"
        )
      )
    );

    syncDownloadPollingForState(get(appState));
    await Promise.all([loadActiveDownloads(), loadCacheSummary()]);
    return true;
  } catch (error) {
    if (!lockAlreadyHeld) {
      releasePackageBusy(request.packageId);
    }

    const fallbackMessage = `Failed to start the install task for ${request.packageName} ${request.versionNumber}.`;
    const errorMessage = error instanceof Error ? `${fallbackMessage} ${error.message}` : fallbackMessage;

    console.error("Failed to queue install task", {
      packageId: request.packageId,
      packageName: request.packageName,
      versionId: request.versionId,
      versionNumber: request.versionNumber,
      effectiveStatus: request.effectiveStatus,
      error
    });

    appState.update((current) => ({
      ...appendActivity(
        current,
        withActivity("Install task failed to start", errorMessage, "warning")
      ),
      downloadError: errorMessage,
      desktopError: current.runtimeKind === "tauri" ? errorMessage : current.desktopError
    }));
    return false;
  }
}

async function switchInstalledPackageVersionInternal(
  request: InstallRequest,
  switchFromVersionIds: string[]
) {
  acquirePackageBusy(request.packageId);
  const currentState = get(appState);
  const installedVersions = findInstalledModsForPackage(
    currentState.activeProfile?.installedMods,
    request.packageId
  )
    .filter((entry) => switchFromVersionIds.includes(entry.versionId))
    .map((entry) => entry.versionNumber);
  const sourceLabel =
    installedVersions.length > 0 ? installedVersions.join(", ") : "installed version";

  let handoffToInstallTask = false;

  try {
    const switched = await uninstallInstalledVersionsForActiveProfile({
      packageId: request.packageId,
      packageName: request.packageName,
      versionIds: switchFromVersionIds,
      title: "Switching version",
      detail: `Removing ${request.packageName} ${sourceLabel} before installing ${request.versionNumber}.`,
      tone: "neutral",
      managePackageLock: false,
      warnOnDependants: false
    });

    if (switched) {
      const queued = await queueVersionForCache(request, { lockAlreadyHeld: true });
      handoffToInstallTask = queued;
      return queued;
    }
  } finally {
    if (!handoffToInstallTask) {
      releasePackageBusy(request.packageId);
    }
  }

  return false;
}

function launchVariantLabel(variant: "modded" | "vanilla") {
  return variant === "modded" ? "Modded" : "Vanilla";
}

function isDependencyPrecheckFailure(result: LaunchResult) {
  return (
    result.code === "PRECHECK_FAILED" &&
    result.message.startsWith("PROFILE_DEPENDENCY_STATE_INVALID:")
  );
}

function buildLaunchFeedback(result: LaunchResult, variant: "modded" | "vanilla") {
  if (result.ok) {
    const modeLabel = result.usedLaunchMode === "direct" ? "Direct" : "Steam";
    return {
      tone: "positive" as const,
      title: `${launchVariantLabel(variant)} launch started`,
      detail:
        result.pid !== undefined
          ? `${modeLabel} launch started (pid ${result.pid}).`
          : `${modeLabel} launch command started.`,
      diagnosticsPath: result.diagnosticsPath,
      canRepair: false,
      canRunAnyway: false
    };
  }

  if (variant === "modded" && isDependencyPrecheckFailure(result)) {
    return {
      tone: "warning" as const,
      title: "Dependency check warning",
      detail:
        "Some enabled mods appear to have dependency issues in the local catalog. If you trust this profile, you can choose Run anyway.",
      diagnosticsPath: result.diagnosticsPath,
      canRepair: false,
      canRunAnyway: true
    };
  }

  return {
    tone: "warning" as const,
    title: `${launchVariantLabel(variant)} launch failed`,
    detail:
      result.code === "STEAM_LAUNCH_OPTIONS_INVALID"
        ? result.message
        : `${result.code}: ${result.message}`,
    diagnosticsPath: result.diagnosticsPath,
    canRepair: shouldOfferRepairForCode(result.code),
    canRunAnyway: false
  };
}

type PostImportHydrationOutcome = {
  referencedUniqueCount: number;
  importedImmediateCount: number;
  queuedCount: number;
  failedQueueCount: number;
  skippedCount: number;
  failedQueueDetails: string[];
};

function packVersionKey(packageId: string, versionId: string) {
  return `${packageId}::${versionId}`;
}

function derivePostImportHydrationMods(
  previewMods: ImportProfilePackPreviewModDto[],
  installedMods: ProfileInstalledModDto[]
) {
  const installedKeys = new Set(
    installedMods
      .map((entry) => packVersionKey(entry.packageId.trim(), entry.versionId.trim()))
      .filter((key) => key !== "::")
  );
  const seenReferencedKeys = new Set<string>();
  const hydrationMods: ImportProfilePackPreviewModDto[] = [];
  let invalidEntryCount = 0;
  let duplicateEntryCount = 0;

  for (const mod of previewMods) {
    const packageId = mod.packageId.trim();
    const versionId = mod.versionId.trim();
    if (!packageId || !versionId) {
      invalidEntryCount += 1;
      continue;
    }

    const key = packVersionKey(packageId, versionId);
    if (seenReferencedKeys.has(key)) {
      duplicateEntryCount += 1;
      continue;
    }

    seenReferencedKeys.add(key);
    if (installedKeys.has(key)) {
      continue;
    }

    hydrationMods.push({
      ...mod,
      packageId,
      versionId
    });
  }

  return {
    hydrationMods,
    referencedUniqueCount: seenReferencedKeys.size,
    importedImmediateCount: seenReferencedKeys.size - hydrationMods.length,
    skippedCount: invalidEntryCount + duplicateEntryCount
  };
}

async function queuePostImportHydration(
  preview: ImportProfilePackPreviewResult,
  importedProfile: ProfileDetailDto
): Promise<PostImportHydrationOutcome> {
  const hydrationPlan = derivePostImportHydrationMods(
    preview.mods ?? [],
    importedProfile.installedMods
  );
  let queuedCount = 0;
  const failedQueueDetails: string[] = [];

  for (const mod of hydrationPlan.hydrationMods) {
    try {
      await queueInstallToCache({
        packageId: mod.packageId,
        versionId: mod.versionId,
        profileId: importedProfile.id
      });
      queuedCount += 1;
    } catch (error) {
      const queueError = describeUnknownError(
        error,
        `Failed to queue hydration for ${mod.packageName} ${mod.versionNumber}.`
      );
      failedQueueDetails.push(`${mod.packageName} ${mod.versionNumber}: ${queueError}`);
      console.error("Failed to queue post-import hydration install", {
        profileId: importedProfile.id,
        packageId: mod.packageId,
        versionId: mod.versionId,
        error
      });
    }
  }

  if (queuedCount > 0) {
    await Promise.all([loadActiveDownloads(), loadCacheSummary()]);
  }

  const outcome = {
    referencedUniqueCount: hydrationPlan.referencedUniqueCount,
    importedImmediateCount: hydrationPlan.importedImmediateCount,
    queuedCount,
    failedQueueCount: failedQueueDetails.length,
    skippedCount: hydrationPlan.skippedCount,
    failedQueueDetails
  };

  console.info("[profile-pack] hydration queue outcomes", {
    profileId: importedProfile.id,
    sourcePath: preview.sourcePath,
    payloadMode: preview.payloadMode,
    referencedUniqueCount: outcome.referencedUniqueCount,
    importedImmediateCount: outcome.importedImmediateCount,
    queuedCount: outcome.queuedCount,
    failedQueueCount: outcome.failedQueueCount,
    skippedCount: outcome.skippedCount
  });
  if (outcome.failedQueueCount > 0) {
    console.warn("[profile-pack] hydration queue failures", {
      profileId: importedProfile.id,
      failures: outcome.failedQueueDetails
    });
  }

  return outcome;
}

function buildImportHydrationActivityDetail(
  profileName: string,
  outcome: PostImportHydrationOutcome
) {
  const failedOrSkippedCount = outcome.failedQueueCount + outcome.skippedCount;
  let detail = `${profileName} was imported from .49pack.`;
  detail += ` Imported immediately from pack payload: ${outcome.importedImmediateCount}.`;
  detail += ` Queued for hydration: ${outcome.queuedCount}.`;
  detail += ` Failed/skipped: ${failedOrSkippedCount}.`;

  if (outcome.failedQueueCount > 0) {
    const previewLimit = 2;
    const previewDetails = outcome.failedQueueDetails.slice(0, previewLimit).join("; ");
    const remaining = outcome.failedQueueCount - previewLimit;
    detail += ` Queue failures: ${previewDetails}${remaining > 0 ? `; +${remaining} more` : ""}.`;
  }

  return {
    detail,
    tone: failedOrSkippedCount > 0 ? ("warning" as const) : ("positive" as const)
  };
}

async function importProfilePackUsingPreview(preview: ImportProfilePackPreviewResult) {
  const sourcePath = preview.sourcePath?.trim();
  if (!sourcePath) {
    throw new Error("Selected .49pack source path is missing.");
  }

  const result = await importProfilePackFromPathApi(sourcePath);
  if (result.cancelled || !result.profile) {
    return null;
  }

  const hydrationOutcome = await queuePostImportHydration(preview, result.profile);
  const [profiles, profilesStorageSummary] = await Promise.all([
    listProfilesApi(),
    getProfilesStorageSummaryApi()
  ]);
  return {
    importedProfile: result.profile,
    profiles,
    profilesStorageSummary,
    hydrationOutcome
  };
}

function buildImportProfileModZipActivityDetail(result: ImportProfileModZipResult): string {
  const importedMod = result.importedMod;
  if (!importedMod) {
    return "Imported mod archive into the active profile.";
  }

  return `${importedMod.packageName} ${importedMod.versionNumber} was imported from .zip.`;
}

async function exportProfilePackWithActivity(options: {
  profileId: string;
  fallbackProfileName: string;
  fallbackModCount: number;
  embedUnavailablePayloads: boolean;
}) {
  const result = await exportProfilePackApi({
    profileId: options.profileId,
    embedUnavailablePayloads: options.embedUnavailablePayloads
  });
  if (result.cancelled) {
    return { cancelled: true as const };
  }

  const exportedName = result.profileName ?? options.fallbackProfileName;
  const exportedModCount = result.modCount ?? options.fallbackModCount;
  const exportedPath = result.path ?? "";

  appState.update((state) =>
    appendActivity(
      {
        ...state,
        profileError: null,
        desktopError: null
      },
      withActivity(
        "Profile exported",
        `${exportedName} was exported as .49pack (${exportedModCount} ${exportedModCount === 1 ? "mod" : "mods"}).${exportedPath ? ` ${exportedPath}` : ""}`,
        "positive"
      )
    )
  );

  return { cancelled: false as const };
}

export const actions = {
  async bootstrap() {
    const runtimeKind = getRuntimeKind();
    clearBusyPackages();
    stopStorageMigrationPolling();

    appState.update((state) => ({
      ...state,
      runtimeKind,
      isBootstrapping: true,
      desktopError: null
    }));

    try {
      const [summary] = await Promise.all([getCatalogSummary()]);

      await loadProfilesState();

      appState.update((state) => ({
        ...state,
        runtimeKind,
        settingsError: null,
        lastCatalogRefreshLabel: summary.lastSyncLabel,
        desktopError: null
      }));

      await Promise.all([
        loadSettingsState(),
        loadProfilesStorageSummary(),
        loadCacheSummary(),
        loadActiveDownloads(),
        loadProtonRuntimesState()
      ]);

      if (!summary.hasCatalog) {
        appState.update((state) => ({
          ...state,
          isCatalogOverlayVisible: true,
          catalogOverlayTitle: "Retrieving Thunderstore catalog",
          catalogOverlayMessage: "Retrieving Thunderstore catalog..."
          ,
          catalogOverlayStep: "network"
        }));
        await refreshCatalog(true, { blockingOverlay: true, includeDependencyWarm: true });
      } else {
        appState.update((state) => ({
          ...state,
          isCatalogOverlayVisible: true,
          catalogOverlayTitle: "Preparing cached catalog",
          catalogOverlayMessage: "Loading the first page of cached results",
          catalogOverlayStep: "browse"
        }));

        await waitForNextPaint();

        const loaded = await loadCatalogFirstPage({
          showLoading: false,
          waitForSelectedPackageDetail: true
        });
        if (!loaded) {
          throw new Error("Failed to load the first page of cached results.");
        }

        await warmDependencyIndexForOverlay();
        await checkForCatalogUpdatesInStartupOverlay();

        appState.update((state) => ({
          ...state,
          isCatalogOverlayVisible: false,
          catalogOverlayTitle: null,
          catalogOverlayMessage: null,
          catalogOverlayStep: null
        }));
      }

    } catch (error) {
      appState.update((state) => ({
        ...state,
        catalogError: error instanceof Error ? error.message : "Failed to bootstrap backend data.",
        isCatalogOverlayVisible: false,
        catalogOverlayTitle: null,
        catalogOverlayMessage: null,
        catalogOverlayStep: null,
        desktopError:
          runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to start the desktop backend."
            : null
      }));
    } finally {
      appState.update((state) => ({
        ...state,
        isBootstrapping: false
      }));
    }
  },
  setView(view: AppView) {
    if (isStorageMigrationBlocking()) {
      return;
    }

    if (view === "browse" && isHeavyWorkBlocked()) {
      notifyHeavyWorkBlocked("Browse view");
      return;
    }

    if (view !== "settings") {
      stopMemoryDiagnosticsPolling();
    }

    appState.update((state) => ({
      ...state,
      view,
      memoryDiagnosticsModal:
        view === "settings" ? state.memoryDiagnosticsModal : null
    }));

    if (view === "settings") {
      void loadProfilesStorageSummary();
    }
  },
  dismissLaunchFeedback() {
    appState.update((state) => ({
      ...state,
      launchFeedback: null
    }));
  },
  async selectProtonRuntime(runtimeId: string) {
    const previous = get(appState).selectedProtonRuntimeId;
    if (!runtimeId || previous === runtimeId) {
      return;
    }

    appState.update((state) => ({
      ...state,
      selectedProtonRuntimeId: runtimeId
    }));

    try {
      await setPreferredProtonRuntimeApi(runtimeId);
      appState.update((state) => ({
        ...state,
        settingsError: null,
        desktopError: null
      }));
    } catch (error) {
      appState.update((state) => ({
        ...state,
        selectedProtonRuntimeId: previous ?? state.selectedProtonRuntimeId,
        settingsError:
          error instanceof Error ? error.message : "Failed to save preferred Proton runtime.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to save preferred desktop Proton runtime."
            : state.desktopError
      }));
    }
  },
  async launchModded() {
    const snapshot = get(appState);
    if (snapshot.isBootstrapping || snapshot.isLaunching) {
      return;
    }

    if (!snapshot.activeProfile) {
      appState.update((state) => ({
        ...state,
        profileError: "No active profile is available for modded launch."
      }));
      return;
    }

    const launchMode = resolveLaunchMode(snapshot.activeProfile);
    appState.update((state) => ({
      ...state,
      isLaunching: true,
      launchingVariant: "modded",
      launchFeedback: {
        tone: "neutral",
        title: "Launching modded profile",
        detail: `Running ${launchMode} preflight and activation...`,
        canRepair: false
      },
      desktopError: null
    }));

    try {
      const result = await launchProfileApi({
        profileId: snapshot.activeProfile.id,
        launchMode,
        protonRuntimeId: snapshot.selectedProtonRuntimeId ?? undefined
      });
      const feedback = buildLaunchFeedback(result, "modded");
      appState.update((state) =>
        appendActivity(
          {
            ...state,
            isLaunching: false,
            launchingVariant: null,
            launchFeedback: feedback
          },
          withActivity(feedback.title, feedback.detail, feedback.tone)
        )
      );
      void loadLaunchRuntimeStatus();
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Launch request failed before the backend could return a result.";
      appState.update((state) =>
        appendActivity(
          {
            ...state,
            isLaunching: false,
            launchingVariant: null,
            launchFeedback: {
              tone: "warning",
              title: "Modded launch failed",
              detail: message,
              canRepair: false
            },
            desktopError: state.runtimeKind === "tauri" ? message : state.desktopError
          },
          withActivity("Modded launch failed", message, "warning")
        )
      );
    }
  },
  async launchModdedRunAnyway() {
    const snapshot = get(appState);
    if (snapshot.isBootstrapping || snapshot.isLaunching) {
      return;
    }

    if (!snapshot.activeProfile) {
      appState.update((state) => ({
        ...state,
        profileError: "No active profile is available for modded launch."
      }));
      return;
    }

    const launchMode = resolveLaunchMode(snapshot.activeProfile);
    appState.update((state) => ({
      ...state,
      isLaunching: true,
      launchingVariant: "modded",
      launchFeedback: {
        tone: "neutral",
        title: "Launching modded profile",
        detail: `Running ${launchMode} launch while skipping dependency precheck...`,
        canRepair: false,
        canRunAnyway: false
      },
      desktopError: null
    }));

    try {
      const result = await launchProfileApi({
        profileId: snapshot.activeProfile.id,
        launchMode,
        protonRuntimeId: snapshot.selectedProtonRuntimeId ?? undefined,
        skipDependencyValidation: true
      });
      const feedback = buildLaunchFeedback(result, "modded");
      appState.update((state) =>
        appendActivity(
          {
            ...state,
            isLaunching: false,
            launchingVariant: null,
            launchFeedback: feedback
          },
          withActivity(feedback.title, feedback.detail, feedback.tone)
        )
      );
      void loadLaunchRuntimeStatus();
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Launch request failed before the backend could return a result.";
      appState.update((state) =>
        appendActivity(
          {
            ...state,
            isLaunching: false,
            launchingVariant: null,
            launchFeedback: {
              tone: "warning",
              title: "Modded launch failed",
              detail: message,
              canRepair: false,
              canRunAnyway: false
            },
            desktopError: state.runtimeKind === "tauri" ? message : state.desktopError
          },
          withActivity("Modded launch failed", message, "warning")
        )
      );
    }
  },
  async launchVanilla() {
    const snapshot = get(appState);
    if (snapshot.isBootstrapping || snapshot.isLaunching) {
      return;
    }

    const launchMode = resolveLaunchMode(snapshot.activeProfile);
    appState.update((state) => ({
      ...state,
      isLaunching: true,
      launchingVariant: "vanilla",
      launchFeedback: {
        tone: "neutral",
        title: "Launching vanilla",
        detail: `Running ${launchMode} cleanup and preflight...`,
        canRepair: false
      },
      desktopError: null
    }));

    try {
      const result = await launchVanillaApi({
        launchMode,
        protonRuntimeId: snapshot.selectedProtonRuntimeId ?? undefined
      });
      const feedback = buildLaunchFeedback(result, "vanilla");
      appState.update((state) =>
        appendActivity(
          {
            ...state,
            isLaunching: false,
            launchingVariant: null,
            launchFeedback: feedback
          },
          withActivity(feedback.title, feedback.detail, feedback.tone)
        )
      );
      void loadLaunchRuntimeStatus();
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Vanilla launch request failed before the backend could return a result.";
      appState.update((state) =>
        appendActivity(
          {
            ...state,
            isLaunching: false,
            launchingVariant: null,
            launchFeedback: {
              tone: "warning",
              title: "Vanilla launch failed",
              detail: message,
              canRepair: false
            },
            desktopError: state.runtimeKind === "tauri" ? message : state.desktopError
          },
          withActivity("Vanilla launch failed", message, "warning")
        )
      );
    }
  },
  async repairLaunchActivation() {
    const snapshot = get(appState);
    if (snapshot.isLaunching) {
      return;
    }

    appState.update((state) => ({
      ...state,
      isLaunching: true,
      launchingVariant: null,
      launchFeedback: {
        tone: "neutral",
        title: "Repairing activation",
        detail: "Cleaning managed files from previous activation...",
        canRepair: false
      }
    }));

    try {
      const result = await repairActivationApi();
      const tone = result.ok ? "positive" : "warning";
      const title = result.ok ? "Activation repaired" : "Activation repair incomplete";
      const detail = `${result.code}: ${result.message}`;

      appState.update((state) =>
        appendActivity(
          {
            ...state,
            isLaunching: false,
            launchingVariant: null,
            launchFeedback: {
              tone,
              title,
              detail,
              diagnosticsPath: state.launchFeedback?.diagnosticsPath,
              canRepair: !result.ok
            }
          },
          withActivity(title, detail, tone)
        )
      );
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to run activation repair.";
      appState.update((state) =>
        appendActivity(
          {
            ...state,
            isLaunching: false,
            launchingVariant: null,
            launchFeedback: {
              tone: "warning",
              title: "Activation repair failed",
              detail: message,
              canRepair: true
            },
            desktopError: state.runtimeKind === "tauri" ? message : state.desktopError
          },
          withActivity("Activation repair failed", message, "warning")
        )
      );
    }
  },
  async openLaunchDiagnostics(path?: string) {
    const diagnosticsPath = path ?? get(appState).launchFeedback?.diagnosticsPath;
    if (!diagnosticsPath) {
      return;
    }

    try {
      await openExternalUrl(toFileUrl(diagnosticsPath));
      appState.update((state) => ({
        ...state,
        desktopError: null
      }));
    } catch (error) {
      appState.update((state) => ({
        ...state,
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to open diagnostics folder from desktop backend."
            : state.desktopError
      }));
    }
  },
  setBrowseSearchDraft(search: string) {
    appState.update((state) => ({
      ...state,
      browseSearchDraft: search
    }));
  },
  async submitBrowseSearch() {
    if (isHeavyWorkBlocked()) {
      notifyHeavyWorkBlocked("Browse search");
      return;
    }

    appState.update((state) => ({
      ...state,
      browseSearchSubmitted: state.browseSearchDraft.trim()
    }));
    await loadCatalogFirstPage();
  },
  async setBrowseSortMode(sortMode: BrowseSortMode) {
    if (isHeavyWorkBlocked()) {
      notifyHeavyWorkBlocked("Browse sorting");
      return;
    }

    appState.update((state) => ({
      ...state,
      browseSortMode: sortMode
    }));
    await loadCatalogFirstPage();
  },
  async toggleVisibleStatus(status: EffectiveStatus) {
    if (isHeavyWorkBlocked()) {
      notifyHeavyWorkBlocked("Browse filtering");
      return;
    }

    appState.update((state) => {
      const visible = state.visibleStatuses.includes(status)
        ? state.visibleStatuses.filter((entry) => entry !== status)
        : [...state.visibleStatuses, status];

      return {
        ...state,
        visibleStatuses: visible
      };
    });

    await loadCatalogFirstPage();
  },
  async selectPackage(packageId: string) {
    if (isHeavyWorkBlocked()) {
      notifyHeavyWorkBlocked("Package details");
      return;
    }

    appState.update((state) => ({
      ...state,
      selectedPackageId: packageId
    }));
    await loadSelectedPackageDetail();
  },
  async loadMoreCatalog() {
    if (isHeavyWorkBlocked()) {
      notifyHeavyWorkBlocked("Browse pagination");
      return;
    }

    await loadCatalogNextPage();
  },
  async selectProfile(profileId: string) {
    try {
      const activeProfile = await setActiveProfileApi(profileId);
      const profiles = await listProfilesApi();
      const profilesStorageSummary = await getProfilesStorageSummaryApi();

      appState.update((state) => ({
        ...mapActiveProfile(state, activeProfile ?? undefined),
        profiles,
        profilesStorageSummary,
        profileError: null,
        desktopError: null
      }));
    } catch (error) {
      appState.update((state) => ({
        ...state,
        profileError: error instanceof Error ? error.message : "Failed to switch profiles.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to switch desktop profiles."
            : state.desktopError
      }));
    }
  },
  requestInstall(request: InstallRequest, options: InstallActionOptions = {}) {
    void requestInstallWithOptionalSwitch(request, [], {
      includeDependencies: options.includeDependencies ?? true,
      promptWithoutDependencies: options.includeDependencies === false
    });
  },
  requestSwitchVersion(
    request: InstallRequest,
    switchFromVersionIds: string[],
    options: InstallActionOptions = {}
  ) {
    void requestInstallWithOptionalSwitch(request, switchFromVersionIds, {
      includeDependencies: options.includeDependencies ?? true,
      promptWithoutDependencies: options.includeDependencies === false
    });
  },
  async switchInstalledPackageVersion(request: InstallRequest, switchFromVersionIds: string[]) {
    await switchInstalledPackageVersionInternal(request, switchFromVersionIds);
  },
  async uninstallPackageFromBrowse(packageId: string, packageName: string) {
    const installed = findInstalledModsForPackage(get(appState).activeProfile?.installedMods, packageId);

    if (installed.length === 0) {
      return;
    }

    const detail =
      installed.length === 1
        ? `${packageName} ${installed[0].versionNumber} was removed from the active profile.`
        : `${installed.length} installed versions of ${packageName} were removed from the active profile.`;

    await uninstallInstalledVersionsForActiveProfile({
      packageId,
      packageName,
      versionIds: installed.map((entry) => entry.versionId),
      title: "Mod uninstalled",
      detail,
      tone: "warning"
    });
  },
  async uninstallVersionFromBrowse(
    packageId: string,
    versionId: string,
    packageName: string,
    versionNumber: string
  ) {
    await uninstallInstalledVersionsForActiveProfile({
      packageId,
      packageName,
      versionIds: [versionId],
      title: "Mod uninstalled",
      detail: `${packageName} ${versionNumber} was removed from the active profile.`,
      tone: "warning"
    });
  },
  dismissModal() {
    appState.update((state) => ({
      ...state,
      modal: null
    }));
    resolvePendingWarningConfirmation(false);
  },
  dismissUninstallDependantsModal() {
    appState.update((state) => ({
      ...state,
      uninstallDependantsModal: null
    }));
    resolvePendingUninstallDependantsConfirmation(false);
  },
  dismissInstallWithoutDependenciesModal() {
    appState.update((state) => ({
      ...state,
      installWithoutDependenciesModal: null
    }));
    resolvePendingInstallWithoutDependenciesConfirmation(false);
  },
  confirmInstallWithoutDependenciesModal() {
    appState.update((state) => ({
      ...state,
      installWithoutDependenciesModal: null
    }));
    resolvePendingInstallWithoutDependenciesConfirmation(true);
  },
  dismissImportModZipModal() {
    appState.update((state) => ({
      ...state,
      importModZipModal: null
    }));
    resolvePendingImportModZipDecision(null);
  },
  confirmImportModZipModal(addToCache: boolean) {
    appState.update((state) => ({
      ...state,
      importModZipModal: null
    }));
    resolvePendingImportModZipDecision(addToCache);
  },
  openMemoryDiagnosticsModal() {
    stopMemoryDiagnosticsPolling();

    appState.update((state) => ({
      ...state,
      memoryDiagnosticsModal: {
        isLoading: true,
        data: state.memoryDiagnosticsModal?.data,
        error: null
      },
      settingsError: null
    }));

    void loadMemoryDiagnosticsSnapshot({ showLoading: true });
    startMemoryDiagnosticsPolling();
  },
  dismissMemoryDiagnosticsModal() {
    stopMemoryDiagnosticsPolling();
    appState.update((state) => ({
      ...state,
      memoryDiagnosticsModal: null
    }));
  },
  refreshMemoryDiagnostics() {
    void loadMemoryDiagnosticsSnapshot({ showLoading: true });
  },
  async requestClearUnreferencedCache() {
    try {
      const preview = await previewClearCacheUnreferenced();

      if (preview.removableCount === 0) {
        appState.update((state) =>
          appendActivity(
            {
              ...state,
              clearUnreferencedCacheModal: null,
              settingsError: null,
              cacheError: null,
              desktopError: null
            },
            withActivity(
              "Cache unchanged",
              "No unreferenced cached mod versions were found to remove.",
              "neutral"
            )
          )
        );
        return;
      }

      appState.update((state) => ({
        ...state,
        clearUnreferencedCacheModal: preview,
        settingsError: null,
        cacheError: null,
        desktopError: null
      }));
    } catch (error) {
      appState.update((state) => ({
        ...state,
        settingsError:
          error instanceof Error
            ? error.message
            : "Failed to prepare unreferenced cache cleanup.",
        cacheError:
          error instanceof Error
            ? error.message
            : "Failed to prepare unreferenced cache cleanup.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to prepare unreferenced cache cleanup in the desktop backend."
            : state.desktopError
      }));
    }
  },
  dismissClearUnreferencedCacheModal() {
    appState.update((state) => ({
      ...state,
      clearUnreferencedCacheModal: null
    }));
  },
  async confirmClearUnreferencedCacheModal() {
    const preview = get(appState).clearUnreferencedCacheModal;
    if (!preview) {
      return;
    }

    try {
      const cacheSummary = await clearCacheUnreferenced();
      stopDownloadPolling();
      clearBusyPackages();
      appState.update((state) =>
        appendActivity(
          {
            ...state,
            clearUnreferencedCacheModal: null,
            cacheSummary,
            downloads: [],
            activeCacheTaskIds: [],
            downloadError: null,
            cacheError: null,
            settingsError: null,
            desktopError: null
          },
          withActivity(
            "Cache cleaned",
            `Removed ${preview.removableCount} cached mod ${preview.removableCount === 1 ? "archive" : "archives"} not installed in any profile.`,
            "warning"
          )
        )
      );
    } catch (error) {
      appState.update((state) => ({
        ...state,
        settingsError:
          error instanceof Error ? error.message : "Failed to clear unreferenced cache entries.",
        cacheError:
          error instanceof Error ? error.message : "Failed to clear unreferenced cache entries.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to clear unreferenced cache entries in the desktop backend."
            : state.desktopError
      }));
    }
  },
  openDependencyModal(request: {
    packageId: string;
    packageName: string;
    versionId: string;
    versionNumber: string;
  }) {
    if (isHeavyWorkBlocked()) {
      notifyHeavyWorkBlocked("Dependency details");
      return;
    }

    appState.update((state) => ({
      ...state,
      dependencyModal: {
        ...request,
        isLoading: true,
        error: null
      }
    }));

    void getVersionDependencies({
      packageId: request.packageId,
      versionId: request.versionId
    })
      .then((data) => {
        appState.update((state) =>
          dependencyModalMatches(state.dependencyModal, request.packageId, request.versionId)
            ? {
                ...state,
                dependencyModal: {
                  ...request,
                  isLoading: false,
                  data,
                  error: null
                }
              }
            : state
        );
      })
      .catch((error) => {
        appState.update((state) =>
          dependencyModalMatches(state.dependencyModal, request.packageId, request.versionId)
            ? {
                ...state,
                dependencyModal: {
                  ...request,
                  isLoading: false,
                  error:
                    error instanceof Error
                      ? error.message
                      : "Failed to resolve dependencies from the cached catalog."
                },
                desktopError:
                  state.runtimeKind === "tauri"
                    ? error instanceof Error
                      ? error.message
                      : "Failed to resolve dependencies from the desktop backend."
                    : state.desktopError
              }
            : state
        );
      });
  },
  closeDependencyModal() {
    appState.update((state) => ({
      ...state,
      dependencyModal: null
    }));
  },
  async jumpToDependency(packageId: string, versionId: string) {
    if (isHeavyWorkBlocked()) {
      notifyHeavyWorkBlocked("Dependency navigation");
      return;
    }

    await navigateToPackageVersionInBrowse(packageId, versionId);
  },
  async jumpToInstalledModDetails(packageId: string, versionId: string) {
    if (isHeavyWorkBlocked()) {
      notifyHeavyWorkBlocked("Installed mod details");
      return;
    }

    await navigateToPackageVersionInBrowse(packageId, versionId);
  },
  confirmModal(doNotShowAgain: boolean) {
    const modal = get(appState).modal;

    appState.update((state) => {
      if (!state.modal) {
        return state;
      }

      const nextState = {
        ...state,
        warningPrefs: {
          red: state.modal.status === "red" && doNotShowAgain ? false : state.warningPrefs.red,
          broken:
            state.modal.status === "broken" && doNotShowAgain ? false : state.warningPrefs.broken,
          installWithoutDependencies: state.warningPrefs.installWithoutDependencies,
          uninstallWithDependants: state.warningPrefs.uninstallWithDependants,
          importProfilePack: state.warningPrefs.importProfilePack,
          conserveWhileGameRunning: state.warningPrefs.conserveWhileGameRunning
        }
      };

      return {
        ...nextState,
        modal: null
      };
    });

    if (doNotShowAgain && modal?.status) {
      void setWarningPreferenceApi(modal.status, false).then((prefs) => {
        appState.update((state) => ({
          ...state,
          warningPrefs: prefs
        }));
      });
    }

    resolvePendingWarningConfirmation(true);
  },
  confirmUninstallDependantsModal(doNotShowAgain: boolean) {
    appState.update((state) => ({
      ...state,
      uninstallDependantsModal: null,
      warningPrefs: {
        ...state.warningPrefs,
        uninstallWithDependants: doNotShowAgain ? false : state.warningPrefs.uninstallWithDependants
      }
    }));

    if (doNotShowAgain) {
      void setWarningPreferenceApi("uninstallWithDependants", false).then((prefs) => {
        appState.update((state) => ({
          ...state,
          warningPrefs: prefs
        }));
      });
    }

    resolvePendingUninstallDependantsConfirmation(true);
  },
  async setWarningPreference(
    kind:
      | "red"
      | "broken"
      | "installWithoutDependencies"
      | "uninstallWithDependants"
      | "importProfilePack"
      | "conserveWhileGameRunning",
    enabled: boolean
  ) {
    try {
      const prefs = await setWarningPreferenceApi(kind, enabled);
      appState.update((state) => ({
        ...state,
        warningPrefs: prefs,
        settingsError: null,
        desktopError: null
      }));
      syncLaunchRuntimePollingForState(get(appState));
      syncDownloadPollingForState(get(appState));
    } catch (error) {
      appState.update((state) => ({
        ...state,
        settingsError: error instanceof Error ? error.message : "Failed to save warning settings.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to save desktop settings."
            : state.desktopError
      }));
    }
  },
  setReferenceSearchDraft(search: string) {
    appState.update((state) => ({
      ...state,
      referenceSearchDraft: search
    }));
  },
  async submitReferenceSearch() {
    if (isHeavyWorkBlocked()) {
      notifyHeavyWorkBlocked("Reference search");
      return;
    }

    appState.update((state) => ({
      ...state,
      referenceSearchSubmitted: state.referenceSearchDraft.trim(),
      referenceRowsData: [],
      referenceNextCursor: null,
      referenceHasMore: false
    }));
    await loadReferenceLibrary();
  },
  async loadMoreReferences() {
    if (isHeavyWorkBlocked()) {
      notifyHeavyWorkBlocked("Reference pagination");
      return;
    }

    await loadMoreReferenceLibrary();
  },
  async refreshCatalog() {
    if (isHeavyWorkBlocked()) {
      notifyHeavyWorkBlocked("Catalog refresh");
      return;
    }

    await refreshCatalog(true, { blockingOverlay: true });
  },
  async toggleInstalledMod(
    profileId: string,
    packageId: string,
    versionId: string,
    enabled: boolean
  ) {
    const targetMod = get(appState).activeProfile?.installedMods.find(
      (entry) => entry.packageId === packageId && entry.versionId === versionId
    );
    const modLabel = targetMod
      ? `${targetMod.packageName} ${targetMod.versionNumber}`
      : `${packageId}:${versionId}`;

    try {
      const [activeProfile, profiles] = await Promise.all([
        setInstalledModEnabledApi({
          profileId,
          packageId,
          versionId,
          enabled
        }),
        listProfilesApi()
      ]);

      appState.update((state) =>
        appendActivity(
          {
            ...mapActiveProfile(state, activeProfile),
            profiles,
            profileError: null,
            desktopError: null
          },
          withActivity(
            enabled ? "Mod enabled" : "Mod disabled",
            `${modLabel} is now ${enabled ? "enabled" : "disabled"} in the active profile.`,
            "neutral"
          )
        )
      );
    } catch (error) {
      appState.update((state) => ({
        ...state,
        profileError: error instanceof Error ? error.message : "Failed to update mod state.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to update mod state in the desktop backend."
            : state.desktopError
      }));
    }
  },
  async uninstallInstalledMod(profileId: string, packageId: string, versionId: string) {
    if (get(appState).activeProfile?.id !== profileId) {
      return;
    }

    const targetMod = get(appState).activeProfile?.installedMods.find(
      (entry) => entry.packageId === packageId && entry.versionId === versionId
    );
    const packageName = targetMod?.packageName ?? packageId;
    const versionNumber = targetMod?.versionNumber ?? versionId;

    await uninstallInstalledVersionsForActiveProfile({
      packageId,
      packageName,
      versionIds: [versionId],
      title: "Mod uninstalled",
      detail: `${packageName} ${versionNumber} was removed from the active profile.`,
      tone: "warning",
      warnOnDependants: true
    });
  },
  async createProfile(input: CreateProfileInput) {
    try {
      const activeProfile = await createProfileApi(input);
      const profiles = await listProfilesApi();
      const profilesStorageSummary = await getProfilesStorageSummaryApi();

      appState.update((state) =>
        appendActivity(
          {
            ...mapActiveProfile(state, activeProfile),
            profiles,
            profilesStorageSummary,
            profileError: null,
            desktopError: null
          },
          withActivity(
            "Profile created",
            `${activeProfile.name} is now the active profile.`,
            "positive"
          )
        )
      );
    } catch (error) {
      appState.update((state) => ({
        ...state,
        profileError: error instanceof Error ? error.message : "Failed to create the profile.",
        desktopError: state.desktopError
      }));
    }
  },
  async updateProfile(input: { profileId: string; name: string; notes?: string; gamePath?: string; launchModeDefault?: "steam" | "direct" }) {
    try {
      const activeProfile = await updateProfileApi(input);
      const profiles = await listProfilesApi();
      const profilesStorageSummary = await getProfilesStorageSummaryApi();

      appState.update((state) =>
        appendActivity(
          {
            ...mapActiveProfile(state, activeProfile),
            profiles,
            profilesStorageSummary,
            profileError: null,
            desktopError: null
          },
          withActivity(
            "Profile updated",
            `${activeProfile.name} was updated.`,
            "neutral"
          )
        )
      );
    } catch (error) {
      appState.update((state) => ({
        ...state,
        profileError: error instanceof Error ? error.message : "Failed to update the profile.",
        desktopError: state.desktopError
      }));
    }
  },
  async deleteSelectedProfile() {
    const selectedProfile = get(appState).activeProfile;

    if (!selectedProfile) {
      return;
    }

    try {
      await deleteProfileApi(selectedProfile.id);
      const [profiles, activeProfile] = await Promise.all([listProfilesApi(), getActiveProfileApi()]);
      const profilesStorageSummary = await getProfilesStorageSummaryApi();

      appState.update((state) =>
        appendActivity(
          {
            ...mapActiveProfile(state, activeProfile ?? undefined),
            profiles,
            profilesStorageSummary,
            profileError: null,
            desktopError: null
          },
          withActivity(
            "Profile deleted",
            `${selectedProfile.name} was removed.`,
            "warning"
          )
        )
      );
    } catch (error) {
      appState.update((state) => ({
        ...state,
        profileError: error instanceof Error ? error.message : "Failed to delete the profile.",
        desktopError: state.desktopError
      }));
    }
  },
  async exportActiveProfilePack() {
    const selectedProfile = get(appState).activeProfile;
    if (!selectedProfile) {
      appState.update((state) => ({
        ...state,
        profileError: "No active profile is selected for export."
      }));
      return;
    }

    try {
      const preview = await previewExportProfilePackApi(selectedProfile.id);
      const unavailableMods = preview.unavailableMods ?? [];
      if (unavailableMods.length > 0) {
        appState.update((state) => ({
          ...state,
          exportProfilePackModal: {
            preview: {
              ...preview,
              unavailableMods
            },
            isExporting: false
          },
          profileError: null,
          desktopError: null
        }));
        return;
      }

      await exportProfilePackWithActivity({
        profileId: selectedProfile.id,
        fallbackProfileName: preview.profileName,
        fallbackModCount: preview.modCount,
        embedUnavailablePayloads: false
      });
    } catch (error) {
      appState.update((state) => ({
        ...state,
        profileError: error instanceof Error ? error.message : "Failed to export the profile.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to export the desktop profile pack."
            : state.desktopError
      }));
    }
  },
  dismissExportProfilePackModal() {
    appState.update((state) =>
      state.exportProfilePackModal?.isExporting
        ? state
        : {
            ...state,
            exportProfilePackModal: null
          }
    );
  },
  async confirmExportProfilePackModal(embedUnavailablePayloads: boolean) {
    const modalState = get(appState).exportProfilePackModal;
    if (!modalState || modalState.isExporting) {
      return;
    }

    appState.update((state) => ({
      ...state,
      exportProfilePackModal: state.exportProfilePackModal
        ? {
            ...state.exportProfilePackModal,
            isExporting: true
          }
        : null,
      profileError: null,
      desktopError: null
    }));

    try {
      await exportProfilePackWithActivity({
        profileId: modalState.preview.profileId,
        fallbackProfileName: modalState.preview.profileName,
        fallbackModCount: modalState.preview.modCount,
        embedUnavailablePayloads
      });
      appState.update((state) => ({
        ...state,
        exportProfilePackModal: null
      }));
    } catch (error) {
      appState.update((state) => ({
        ...state,
        exportProfilePackModal: null,
        profileError: error instanceof Error ? error.message : "Failed to export the profile.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to export the desktop profile pack."
            : state.desktopError
      }));
    }
  },
  async importModZipToActiveProfile() {
    if (isHeavyWorkBlocked()) {
      notifyHeavyWorkBlocked("Import mod");
      return;
    }

    const selectedProfile = get(appState).activeProfile;
    if (!selectedProfile) {
      appState.update((state) => ({
        ...state,
        profileError: "No active profile is selected for mod import."
      }));
      return;
    }

    try {
      const result = await importProfileModZipApi({
        profileId: selectedProfile.id,
        addToCache: false
      });
      if (result.cancelled || !result.profile) {
        return;
      }

      const [profiles, profilesStorageSummary] = await Promise.all([
        listProfilesApi(),
        getProfilesStorageSummaryApi()
      ]);

      appState.update((state) =>
        appendActivity(
          {
            ...mapActiveProfile(state, result.profile),
            profiles,
            profilesStorageSummary,
            cacheSummary: state.cacheSummary,
            profileError: null,
            cacheError: null,
            desktopError: null
          },
          withActivity(
            "Mod imported",
            buildImportProfileModZipActivityDetail(result),
            "positive"
          )
        )
      );
    } catch (error) {
      appState.update((state) => ({
        ...state,
        profileError: error instanceof Error ? error.message : "Failed to import the mod archive.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to import the desktop mod archive."
            : state.desktopError
      }));
    }
  },
  async importProfilePack() {
    try {
      const preview = await previewImportProfilePackApi();
      if (preview.cancelled) {
        return;
      }

      if (!get(appState).warningPrefs.importProfilePack) {
        const imported = await importProfilePackUsingPreview(preview);
        if (!imported) {
          return;
        }

        const importedProfile = imported.importedProfile;
        const importActivity = buildImportHydrationActivityDetail(
          importedProfile.name,
          imported.hydrationOutcome
        );

        appState.update((state) =>
          appendActivity(
            {
              ...mapActiveProfile(state, importedProfile),
              profiles: imported.profiles,
              profilesStorageSummary: imported.profilesStorageSummary,
              importProfilePackModal: null,
              profileError: null,
              desktopError: null
            },
            withActivity(
              "Profile imported",
              importActivity.detail,
              importActivity.tone
            )
          )
        );
        return;
      }

      appState.update((state) => ({
        ...state,
        importProfilePackModal: {
          preview,
          isImporting: false
        },
        profileError: null,
        desktopError: null
      }));
    } catch (error) {
      appState.update((state) => ({
        ...state,
        profileError: error instanceof Error ? error.message : "Failed to prepare profile import.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to prepare profile import in the desktop backend."
            : state.desktopError
      }));
    }
  },
  dismissImportProfilePackModal() {
    appState.update((state) =>
      state.importProfilePackModal?.isImporting
        ? state
        : {
            ...state,
            importProfilePackModal: null
          }
    );
  },
  async confirmImportProfilePackModal(doNotShowAgain: boolean) {
    const modalState = get(appState).importProfilePackModal;
    if (!modalState || modalState.isImporting) {
      return;
    }

    appState.update((state) => ({
      ...state,
      importProfilePackModal: state.importProfilePackModal
        ? {
            ...state.importProfilePackModal,
            isImporting: true
          }
        : null,
      warningPrefs: {
        ...state.warningPrefs,
        importProfilePack: doNotShowAgain ? false : state.warningPrefs.importProfilePack
      }
    }));

    if (doNotShowAgain) {
      void setWarningPreferenceApi("importProfilePack", false).then((prefs) => {
        appState.update((state) => ({
          ...state,
          warningPrefs: prefs
        }));
      });
    }

    try {
      const imported = await importProfilePackUsingPreview(modalState.preview);
      if (!imported) {
        appState.update((state) => ({
          ...state,
          importProfilePackModal: null
        }));
        return;
      }

      const importedProfile = imported.importedProfile;
      const importActivity = buildImportHydrationActivityDetail(
        importedProfile.name,
        imported.hydrationOutcome
      );

      appState.update((state) =>
        appendActivity(
          {
            ...mapActiveProfile(state, importedProfile),
            profiles: imported.profiles,
            profilesStorageSummary: imported.profilesStorageSummary,
            importProfilePackModal: null,
            profileError: null,
            desktopError: null
          },
          withActivity(
            "Profile imported",
            importActivity.detail,
            importActivity.tone
          )
        )
      );
    } catch (error) {
      appState.update((state) => ({
        ...state,
        importProfilePackModal: null,
        profileError: error instanceof Error ? error.message : "Failed to import the profile.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to import the desktop profile pack."
            : state.desktopError
      }));
    }
  },
  async openCacheFolder() {
    try {
      await openCacheFolder();
      appState.update((state) => ({
        ...state,
        settingsError: null,
        cacheError: null,
        desktopError: null
      }));
    } catch (error) {
      appState.update((state) => ({
        ...state,
        settingsError: error instanceof Error ? error.message : "Failed to open the cache folder.",
        cacheError: error instanceof Error ? error.message : "Failed to open the cache folder.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to open the desktop cache folder."
            : state.desktopError
      }));
    }
  },
  async moveStorageLocation(kind: "cache" | "profiles") {
    if (isStorageMigrationBlocking()) {
      return;
    }

    try {
      const selectedPath = await pickStorageFolder(kind);
      if (!selectedPath) {
        return;
      }

      const status = await startStorageMigration(
        kind === "cache"
          ? { cacheDir: selectedPath }
          : { profilesDir: selectedPath }
      );

      stopDownloadPolling();
      stopLaunchRuntimePolling();
      stopMemoryDiagnosticsPolling();

      appState.update((state) =>
        appendActivity(
          {
            ...state,
            storageMigration: status,
            settingsError: null,
            desktopError: null
          },
          withActivity(
            "Storage migration started",
            `Moving ${kind} data to ${selectedPath}.`,
            "neutral"
          )
        )
      );

      startStorageMigrationPolling();
    } catch (error) {
      appState.update((state) => ({
        ...state,
        settingsError:
          error instanceof Error
            ? error.message
            : `Failed to move ${kind} storage location.`,
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : `Failed to move desktop ${kind} storage location.`
            : state.desktopError
      }));
      syncLaunchRuntimePollingForState(get(appState));
      syncDownloadPollingForState(get(appState));
    }
  },
  async openProfilesFolder() {
    try {
      await openProfilesFolderApi();
      appState.update((state) => ({
        ...state,
        settingsError: null,
        desktopError: null
      }));
    } catch (error) {
      appState.update((state) => ({
        ...state,
        settingsError: error instanceof Error ? error.message : "Failed to open the profiles folder.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to open the desktop profiles folder."
            : state.desktopError
      }));
    }
  },
  async openActiveProfileFolder() {
    try {
      await openActiveProfileFolderApi();
      appState.update((state) => ({
        ...state,
        settingsError: null,
        desktopError: null
      }));
    } catch (error) {
      appState.update((state) => ({
        ...state,
        settingsError:
          error instanceof Error ? error.message : "Failed to open the active profile folder.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to open the desktop active profile folder."
            : state.desktopError
      }));
    }
  },
  async clearCache() {
    try {
      const cacheSummary = await clearCache();
      stopDownloadPolling();
      clearBusyPackages();
      appState.update((state) =>
        appendActivity(
          {
            ...state,
            cacheSummary,
            downloads: [],
            activeCacheTaskIds: [],
            downloadError: null,
            cacheError: null,
            settingsError: null,
            desktopError: null
          },
          withActivity(
            "Cache cleared",
            "All cached mod archives were removed from local storage.",
            "warning"
          )
        )
      );
    } catch (error) {
      appState.update((state) => ({
        ...state,
        settingsError: error instanceof Error ? error.message : "Failed to clear the cache.",
        cacheError: error instanceof Error ? error.message : "Failed to clear the cache.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to clear the desktop cache."
            : state.desktopError
      }));
    }
  },
  async resetAllData() {
    if (isStorageMigrationBlocking()) {
      return;
    }

    resolvePendingWarningConfirmation(false);
    resolvePendingUninstallDependantsConfirmation(false);
    resolvePendingInstallWithoutDependenciesConfirmation(false);
    resolvePendingImportModZipDecision(null);

    const setResetProgress = (step: ResetProgressStep, title: string, message: string) => {
      appState.update((state) => ({
        ...state,
        resetProgress: {
          step,
          title,
          message
        },
        settingsError: null,
        desktopError: null
      }));
    };

    try {
      setResetProgress(
        "deleting",
        "Resetting app data",
        "Deleting local profiles, settings, cached metadata, and archives."
      );
      await waitForNextPaint();

      await resetAllDataApi();
      stopDownloadPolling();
      stopMemoryDiagnosticsPolling();
      stopStorageMigrationPolling();
      clearBusyPackages();

      setResetProgress(
        "restoring",
        "Resetting app data",
        "Restoring default profile and warning settings."
      );

      appState.update((state) => ({
        ...state,
        profiles: [],
        activeProfile: undefined,
        selectedProfileId: "default",
        modal: null,
        uninstallDependantsModal: null,
        installWithoutDependenciesModal: null,
        memoryDiagnosticsModal: null,
        clearUnreferencedCacheModal: null,
        exportProfilePackModal: null,
        importProfilePackModal: null,
        importModZipModal: null,
        dependencyModal: null,
        focusedVersion: null,
        catalogCards: [],
        catalogNextCursor: null,
        catalogHasMore: false,
        selectedPackageDetail: undefined,
        lastCatalogRefreshLabel: "Catalog not synced yet",
        catalogError: null,
        referenceError: null,
        profileError: null,
        downloadError: null,
        cacheError: null,
        settingsError: null,
        desktopError: null,
        storageMigration: null,
        downloads: [],
        cacheSummary: undefined,
        storageLocations: undefined,
        profilesStorageSummary: undefined,
        activeCacheTaskIds: [],
        protonRuntimes: [],
        selectedProtonRuntimeId: null,
        isLoadingProtonRuntimes: false,
        isGameRunning: false,
        resourceSaverActive: false,
        resourceSaverLastView: null,
        isLaunching: false,
        launchingVariant: null,
        launchFeedback: null,
        activities: seedActivities,
        isCatalogOverlayVisible: false,
        catalogOverlayTitle: null,
        catalogOverlayMessage: null,
        catalogOverlayStep: null,
        isRefreshingCatalog: false,
        isLoadingCatalogFirstPage: false,
        isLoadingCatalogNextPage: false,
        isLoadingPackageDetail: false,
        isLoadingProfilesStorageSummary: false,
        isLoadingReferences: false,
        isLoadingReferencesNextPage: false,
        referenceRowsData: [],
        referenceNextCursor: null,
        referenceHasMore: false
      }));

      clearFocusedVersionTimer();
      await Promise.all([
        loadProfilesState(),
        loadSettingsState(),
        loadProfilesStorageSummary(),
        loadCacheSummary(),
        loadActiveDownloads(),
        loadProtonRuntimesState()
      ]);

      setResetProgress(
        "browse",
        "Refreshing Browse data",
        "Downloading fresh catalog metadata and rebuilding Browse results."
      );

      await refreshCatalog(true, {
        blockingOverlay: false,
        includeDependencyWarm: true,
        showFirstPageLoading: false,
        waitForSelectedPackageDetail: true
      });

      const refreshedState = get(appState);
      if (refreshedState.catalogError) {
        throw new Error(refreshedState.catalogError);
      }

      setResetProgress("finalizing", "Finalizing reset", "Applying final state updates.");
      await waitForNextPaint();

      appState.update((state) =>
        appendActivity(
          {
            ...state,
            resetProgress: null
          },
          withActivity(
            "App data reset",
            "Local data was reset and Browse was refreshed from Thunderstore.",
            "positive"
          )
        )
      );
    } catch (error) {
      appState.update((state) => ({
        ...state,
        resetProgress: null,
        settingsError: error instanceof Error ? error.message : "Failed to reset app data.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to reset desktop app data."
            : state.desktopError
      }));
    }
  },
  async setReferenceState(packageId: string, versionId: string, referenceState: ReferenceState) {
    if (isHeavyWorkBlocked()) {
      notifyHeavyWorkBlocked("Reference updates");
      return;
    }

    try {
      const pkg = get(appState).packages.find((entry) => entry.id === packageId);
      const version = pkg?.versions.find((entry) => entry.id === versionId);

      await setReferenceStateApi({
        packageId,
        versionId,
        referenceState
      });

      appState.update((state) =>
        appendActivity(
          {
            ...state,
            packages: mergeMockReferenceState(state.packages, packageId, versionId, referenceState)
          },
          withActivity(
            "Reference library updated",
            referenceState === "neutral"
              ? `Cleared local override for ${pkg?.fullName ?? packageId} ${version?.versionNumber ?? versionId}.`
              : `Marked ${pkg?.fullName ?? packageId} ${version?.versionNumber ?? versionId} as ${referenceState}.`,
            referenceState === "broken" ? "warning" : "positive"
          )
        )
      );

      await Promise.all([
        loadCatalogFirstPage(),
        get(appState).view === "settings" ? loadReferenceLibrary() : Promise.resolve()
      ]);
    } catch (error) {
      appState.update((state) => ({
        ...state,
        referenceError:
          error instanceof Error ? error.message : "Failed to update the reference library.",
        desktopError:
          state.runtimeKind === "tauri"
            ? error instanceof Error
              ? error.message
              : "Failed to update the desktop reference library."
            : state.desktopError
      }));
    }
  }
};

export const selectedProfile = derived(appState, ($appState) => $appState.activeProfile);
