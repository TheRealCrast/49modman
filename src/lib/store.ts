import { derived, get, writable } from "svelte/store";
import { clearCache, getCacheSummary, openCacheFolder, queueInstallToCache } from "./api/cache";
import { getCatalogSummary, getPackageDetail, searchPackages, syncCatalog } from "./api/catalog";
import { getVersionDependencies, warmDependencyIndex } from "./api/dependencies";
import { listActiveDownloads } from "./api/downloads";
import {
  launchProfile as launchProfileApi,
  launchVanilla as launchVanillaApi,
  listProtonRuntimes as listProtonRuntimesApi,
  repairActivation as repairActivationApi,
  setPreferredProtonRuntime as setPreferredProtonRuntimeApi
} from "./api/launch";
import {
  createProfile as createProfileApi,
  deleteProfile as deleteProfileApi,
  getActiveProfile as getActiveProfileApi,
  getUninstallDependants as getUninstallDependantsApi,
  getProfilesStorageSummary as getProfilesStorageSummaryApi,
  listProfiles as listProfilesApi,
  openActiveProfileFolder as openActiveProfileFolderApi,
  openProfilesFolder as openProfilesFolderApi,
  resetAllData as resetAllDataApi,
  setActiveProfile as setActiveProfileApi,
  setInstalledModEnabled as setInstalledModEnabledApi,
  uninstallInstalledMod as uninstallInstalledModApi,
  updateProfile as updateProfileApi
} from "./api/profiles";
import { listReferenceRows, setReferenceState as setReferenceStateApi } from "./api/reference";
import {
  getWarningPrefs,
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
  CacheSummaryDto,
  CreateProfileInput,
  DependencySummaryItemDto,
  DependencyModalState,
  DownloadJobDto,
  EffectiveStatus,
  InstallActionOptions,
  FocusedVersionState,
  InstallRequest,
  LaunchMode,
  LaunchResult,
  ModPackage,
  ProfileDetailDto,
  ProfileInstalledModDto,
  ReferenceState,
  ResetProgressStep
} from "./types";

const defaultVisibleStatuses: EffectiveStatus[] = ["verified", "green", "yellow", "orange"];
const defaultCatalogPageSize = 40;
const defaultReferencePageSize = 50;
const downloadPollIntervalMs = 500;
let downloadPollHandle: number | null = null;
let focusedVersionClearHandle: number | null = null;
const busyPackageRefCounts = new Map<string, number>();
const activeInstallTaskPackageIds = new Map<string, string>();
let pendingWarningConfirmationResolver: ((confirmed: boolean) => void) | null = null;
let pendingUninstallDependantsConfirmationResolver: ((confirmed: boolean) => void) | null = null;

const initialState: AppState = {
  view: "browse",
  runtimeKind: getRuntimeKind(),
  browseSearchDraft: "",
  browseSearchSubmitted: "",
  visibleStatuses: defaultVisibleStatuses,
  selectedPackageId: "bepinex-pack",
  selectedProfileId: "default",
  packages: seedPackages,
  profiles: [],
  activeProfile: undefined,
  downloads: [],
  cacheSummary: undefined,
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
    uninstallWithDependants: true
  },
  modal: null,
  uninstallDependantsModal: null,
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

function stopDownloadPolling() {
  if (downloadPollHandle !== null) {
    window.clearInterval(downloadPollHandle);
    downloadPollHandle = null;
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
  const warningPrefs = await getWarningPrefs();
  appState.update((state) => ({
    ...state,
    warningPrefs,
    settingsError: null,
    desktopError: null
  }));
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

    if (activeTaskIds.length > 0) {
      startDownloadPolling();
    } else {
      stopDownloadPolling();
      if (hadActiveTasks) {
        await Promise.all([loadCacheSummary(), refreshActiveProfileState(), loadProfilesStorageSummary()]);
        appendDownloadActivity(previous.downloads, activeTaskIds);
      }
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

function confirmInstallWithoutDependenciesIfNeeded() {
  const state = get(appState);
  if (!state.warningPrefs.installWithoutDependencies) {
    return true;
  }

  return window.confirm(
    "Install without dependencies? The selected mod may fail if required dependencies are missing."
  );
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
    const confirmed = confirmInstallWithoutDependenciesIfNeeded();
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

    startDownloadPolling();
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
      canRepair: false
    };
  }

  return {
    tone: "warning" as const,
    title: `${launchVariantLabel(variant)} launch failed`,
    detail: `${result.code}: ${result.message}`,
    diagnosticsPath: result.diagnosticsPath,
    canRepair: shouldOfferRepairForCode(result.code)
  };
}

export const actions = {
  async bootstrap() {
    const runtimeKind = getRuntimeKind();
    clearBusyPackages();

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
    appState.update((state) => ({
      ...state,
      view
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
    appState.update((state) => ({
      ...state,
      browseSearchSubmitted: state.browseSearchDraft.trim()
    }));
    await loadCatalogFirstPage();
  },
  async toggleVisibleStatus(status: EffectiveStatus) {
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
    appState.update((state) => ({
      ...state,
      selectedPackageId: packageId
    }));
    await loadSelectedPackageDetail();
  },
  async loadMoreCatalog() {
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
  openDependencyModal(request: {
    packageId: string;
    packageName: string;
    versionId: string;
    versionNumber: string;
  }) {
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
    await navigateToPackageVersionInBrowse(packageId, versionId);
  },
  async jumpToInstalledModDetails(packageId: string, versionId: string) {
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
          uninstallWithDependants: state.warningPrefs.uninstallWithDependants
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
    kind: "red" | "broken" | "installWithoutDependencies" | "uninstallWithDependants",
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
    await loadMoreReferenceLibrary();
  },
  async refreshCatalog() {
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
        downloads: [],
        cacheSummary: undefined,
        profilesStorageSummary: undefined,
        activeCacheTaskIds: [],
        protonRuntimes: [],
        selectedProtonRuntimeId: null,
        isLoadingProtonRuntimes: false,
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
