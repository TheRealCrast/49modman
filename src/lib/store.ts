import { derived, get, writable } from "svelte/store";
import { getCatalogSummary, getPackageDetail, searchPackages, syncCatalog } from "./api/catalog";
import { listReferenceRows, setReferenceState as setReferenceStateApi } from "./api/reference";
import {
  getWarningPrefs,
  setWarningPreference as setWarningPreferenceApi
} from "./api/settings";
import { seedActivities, seedDownloads, seedPackages, seedProfiles } from "./mock-data";
import { getRuntimeKind } from "./runtime";
import { resolveEffectiveStatus } from "./status";
import type {
  ActivityItem,
  AppState,
  AppView,
  EffectiveStatus,
  ModPackage,
  Profile,
  ReferenceState
} from "./types";

const defaultVisibleStatuses: EffectiveStatus[] = ["verified", "green", "yellow", "orange"];
const defaultCatalogPageSize = 40;
const defaultReferencePageSize = 50;

const initialState: AppState = {
  view: "browse",
  runtimeKind: getRuntimeKind(),
  browseSearchDraft: "",
  browseSearchSubmitted: "",
  visibleStatuses: defaultVisibleStatuses,
  selectedPackageId: "bepinex-pack",
  selectedProfileId: "crew-v49",
  packages: seedPackages,
  profiles: seedProfiles,
  downloads: seedDownloads,
  activities: seedActivities,
  warningPrefs: {
    red: true,
    broken: true
  },
  modal: null,
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

function withActivity(title: string, detail: string, tone: ActivityItem["tone"]) {
  return {
    id: `activity-${Date.now()}`,
    title,
    detail,
    tone
  };
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

async function loadCatalogFirstPage(options: { showLoading?: boolean } = {}) {
  const { showLoading = true } = options;
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
      void loadSelectedPackageDetail(nextSelection);
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

async function refreshCatalog(force: boolean, options: { blockingOverlay?: boolean } = {}) {
  const { blockingOverlay = false } = options;

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
    const reloaded = await loadCatalogFirstPage({ showLoading: !blockingOverlay });

    if (!reloaded) {
      throw new Error("The catalog cache refreshed, but the first page could not be loaded.");
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

function installVersion(state: AppState, packageId: string, versionId: string): AppState {
  const selectedProfile = state.profiles.find((profile) => profile.id === state.selectedProfileId);
  const pkg = findPackage(state, packageId);
  const version = findPackage(state, packageId)?.versions.find((entry) => entry.id === versionId);

  if (!selectedProfile || !pkg || !version) {
    return state;
  }

  const alreadyInstalled = selectedProfile.installedMods.some(
    (mod) => mod.packageId === packageId && mod.versionId === versionId
  );

  const nextProfiles = state.profiles.map((profile) =>
    profile.id !== selectedProfile.id || alreadyInstalled
      ? profile
      : {
          ...profile,
          installedMods: [
            {
              packageId,
              versionId,
              enabled: true
            },
            ...profile.installedMods
          ]
        }
  );

  const nextDownloads = [
    {
      id: `${packageId}-${versionId}-${Date.now()}`,
      packageName: pkg.fullName,
      versionNumber: version.versionNumber,
      progressLabel: "Ready for cache-backed install once download services land",
      status: "queued" as const,
      speedLabel: "backend install pending",
      cacheHit: false
    },
    ...state.downloads
  ].slice(0, 8);

  return appendActivity(
    {
      ...state,
      profiles: nextProfiles,
      downloads: nextDownloads,
      modal: null
    },
    withActivity(
      `Queued ${pkg.fullName} ${version.versionNumber}`,
      `${resolveEffectiveStatus(version)} version added to ${selectedProfile.name}. Metadata is now backend-backed; install execution still uses the frontend mock.`,
      resolveEffectiveStatus(version) === "broken" || resolveEffectiveStatus(version) === "red"
        ? "warning"
        : "positive"
    )
  );
}

export const actions = {
  async bootstrap() {
    const runtimeKind = getRuntimeKind();

    appState.update((state) => ({
      ...state,
      runtimeKind,
      isBootstrapping: true,
      desktopError: null
    }));

    try {
      const [warningPrefs, summary] = await Promise.all([getWarningPrefs(), getCatalogSummary()]);

      appState.update((state) => ({
        ...state,
        runtimeKind,
        warningPrefs,
        settingsError: null,
        lastCatalogRefreshLabel: summary.lastSyncLabel,
        desktopError: null
      }));

      if (!summary.hasCatalog) {
        appState.update((state) => ({
          ...state,
          isCatalogOverlayVisible: true,
          catalogOverlayTitle: "Retrieving Thunderstore catalog",
          catalogOverlayMessage: "Retrieving Thunderstore catalog..."
          ,
          catalogOverlayStep: "network"
        }));
        await refreshCatalog(true, { blockingOverlay: true });
      } else {
        await loadCatalogFirstPage();
        void refreshCatalog(false);
      }

    } catch (error) {
      appState.update((state) => ({
        ...state,
        catalogError: error instanceof Error ? error.message : "Failed to bootstrap backend data.",
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

    const state = get(appState);
    if (view === "settings" && state.referenceRowsData.length === 0 && !state.isLoadingReferences) {
      void loadReferenceLibrary();
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
  selectProfile(profileId: string) {
    appState.update((state) => ({
      ...state,
      selectedProfileId: profileId
    }));
  },
  requestInstall(packageId: string, versionId: string) {
    appState.update((state) => {
      const version =
        state.selectedPackageDetail?.id === packageId
          ? state.selectedPackageDetail.versions.find((entry) => entry.id === versionId)
          : undefined;
      const effectiveStatus = version?.effectiveStatus ?? version?.baseZone;

      if (effectiveStatus === "broken" && state.warningPrefs.broken) {
        return {
          ...state,
          modal: {
            packageId,
            versionId,
            status: "broken"
          }
        };
      }

      if (effectiveStatus === "red" && state.warningPrefs.red) {
        return {
          ...state,
          modal: {
            packageId,
            versionId,
            status: "red"
          }
        };
      }

      return installVersion(state, packageId, versionId);
    });
  },
  dismissModal() {
    appState.update((state) => ({
      ...state,
      modal: null
    }));
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
            state.modal.status === "broken" && doNotShowAgain ? false : state.warningPrefs.broken
        }
      };

      return installVersion(nextState, state.modal.packageId, state.modal.versionId);
    });

    if (doNotShowAgain && modal?.status) {
      void setWarningPreferenceApi(modal.status, false).then((prefs) => {
        appState.update((state) => ({
          ...state,
          warningPrefs: prefs
        }));
      });
    }
  },
  async setWarningPreference(kind: "red" | "broken", enabled: boolean) {
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
  toggleInstalledMod(packageId: string, versionId: string) {
    appState.update((state) => {
      const selectedProfile = state.profiles.find((profile) => profile.id === state.selectedProfileId);

      if (!selectedProfile) {
        return state;
      }

      const nextProfiles = state.profiles.map((profile) =>
        profile.id !== selectedProfile.id
          ? profile
          : {
              ...profile,
              installedMods: profile.installedMods.map((mod) =>
                mod.packageId === packageId && mod.versionId === versionId
                  ? { ...mod, enabled: !mod.enabled }
                  : mod
              )
            }
      );

      const pkg = findPackage(state, packageId);
      const version = findPackage(state, packageId)?.versions.find((entry) => entry.id === versionId);
      const toggledMod = selectedProfile.installedMods.find(
        (mod) => mod.packageId === packageId && mod.versionId === versionId
      );

      if (!pkg || !version || !toggledMod) {
        return state;
      }

      return appendActivity(
        {
          ...state,
          profiles: nextProfiles
        },
        withActivity(
          toggledMod.enabled ? "Mod disabled" : "Mod enabled",
          `${pkg.fullName} ${version.versionNumber} was ${toggledMod.enabled ? "disabled" : "enabled"} in ${selectedProfile.name}.`,
          "neutral"
        )
      );
    });
  },
  uninstallInstalledMod(packageId: string, versionId: string) {
    appState.update((state) => {
      const selectedProfile = state.profiles.find((profile) => profile.id === state.selectedProfileId);

      if (!selectedProfile) {
        return state;
      }

      const nextProfiles = state.profiles.map((profile) =>
        profile.id !== selectedProfile.id
          ? profile
          : {
              ...profile,
              installedMods: profile.installedMods.filter(
                (mod) => !(mod.packageId === packageId && mod.versionId === versionId)
              )
            }
      );

      const pkg = findPackage(state, packageId);
      const version = findPackage(state, packageId)?.versions.find((entry) => entry.id === versionId);

      if (!pkg || !version) {
        return state;
      }

      return appendActivity(
        {
          ...state,
          profiles: nextProfiles
        },
        withActivity(
          "Mod uninstalled",
          `${pkg.fullName} ${version.versionNumber} was removed from ${selectedProfile.name}.`,
          "warning"
        )
      );
    });
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

export const selectedProfile = derived(appState, ($appState) =>
  $appState.profiles.find((profile) => profile.id === $appState.selectedProfileId)
);
