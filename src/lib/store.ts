import { derived, get, writable } from "svelte/store";
import { getCatalogSummary, getPackageDetail, searchPackages, syncCatalog } from "./api/catalog";
import { listReferenceRows, setReferenceState as setReferenceStateApi } from "./api/reference";
import {
  getWarningPrefs,
  setWarningPreference as setWarningPreferenceApi
} from "./api/settings";
import { seedActivities, seedDownloads, seedPackages, seedProfiles } from "./mock-data";
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

const initialState: AppState = {
  view: "browse",
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
  isLoadingPackageDetail: false,
  isLoadingReferences: false,
  lastCatalogRefreshLabel: "Cached mod list ready",
  catalogCards: [],
  selectedPackageDetail: undefined,
  referenceRowsData: [],
  catalogError: null,
  referenceError: null,
  settingsError: null
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

async function loadCatalogCards() {
  const state = get(appState);

  try {
    const cards = await searchPackages({
      query: state.browseSearchSubmitted.trim(),
      visibleStatuses: state.visibleStatuses
    });

    appState.update((current) => {
      const fallbackSelection = cards[0]?.id;
      const selectedPackageId = cards.some((card) => card.id === current.selectedPackageId)
        ? current.selectedPackageId
        : fallbackSelection ?? current.selectedPackageId;

      return {
        ...current,
        catalogCards: cards,
        selectedPackageId,
        catalogError: null
      };
    });
  } catch (error) {
    appState.update((current) => ({
      ...current,
      catalogError: error instanceof Error ? error.message : "Failed to load the catalog."
    }));
  }
}

async function loadSelectedPackageDetail() {
  const state = get(appState);

  if (!state.selectedPackageId) {
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
    const detail = await getPackageDetail(state.selectedPackageId);

    appState.update((current) => ({
      ...current,
      selectedPackageDetail: detail ?? undefined,
      isLoadingPackageDetail: false,
      catalogError: detail ? current.catalogError : "Selected package is no longer available."
    }));
  } catch (error) {
    appState.update((current) => ({
      ...current,
      isLoadingPackageDetail: false,
      catalogError: error instanceof Error ? error.message : "Failed to load package details."
    }));
  }
}

async function loadReferenceLibrary() {
  const state = get(appState);

  appState.update((current) => ({
    ...current,
    isLoadingReferences: true
  }));

  try {
    const rows = await listReferenceRows(state.referenceSearchSubmitted.trim());

    appState.update((current) => ({
      ...current,
      referenceRowsData: rows,
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

async function refreshCatalog(force: boolean) {
  appState.update((state) => ({
    ...state,
    isRefreshingCatalog: true,
    catalogError: null,
    lastCatalogRefreshLabel: force ? "Refreshing cached mod list..." : state.lastCatalogRefreshLabel
  }));

  try {
    const result = await syncCatalog({ force });
    const summary = await getCatalogSummary();
    await loadCatalogCards();
    await loadSelectedPackageDetail();

    appState.update((state) =>
      appendActivity(
        {
          ...state,
          isRefreshingCatalog: false,
          lastCatalogRefreshLabel: result.outcome === "synced" ? result.message : summary.lastSyncLabel
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
      catalogError: error instanceof Error ? error.message : "Failed to refresh the cached mod list."
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
    appState.update((state) => ({
      ...state,
      isBootstrapping: true
    }));

    try {
      const [warningPrefs, summary] = await Promise.all([getWarningPrefs(), getCatalogSummary()]);

      appState.update((state) => ({
        ...state,
        warningPrefs,
        settingsError: null,
        lastCatalogRefreshLabel: summary.lastSyncLabel
      }));

      if (!summary.hasCatalog) {
        await refreshCatalog(true);
      } else {
        await loadCatalogCards();
        await loadSelectedPackageDetail();
        void refreshCatalog(false);
      }

      await loadReferenceLibrary();
    } catch (error) {
      appState.update((state) => ({
        ...state,
        catalogError: error instanceof Error ? error.message : "Failed to bootstrap backend data."
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
    await loadCatalogCards();
    await loadSelectedPackageDetail();
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

    await loadCatalogCards();
    await loadSelectedPackageDetail();
  },
  async selectPackage(packageId: string) {
    appState.update((state) => ({
      ...state,
      selectedPackageId: packageId
    }));
    await loadSelectedPackageDetail();
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
        settingsError: null
      }));
    } catch (error) {
      appState.update((state) => ({
        ...state,
        settingsError: error instanceof Error ? error.message : "Failed to save warning settings."
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
      referenceSearchSubmitted: state.referenceSearchDraft.trim()
    }));
    await loadReferenceLibrary();
  },
  async refreshCatalog() {
    await refreshCatalog(true);
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

      await Promise.all([loadCatalogCards(), loadSelectedPackageDetail(), loadReferenceLibrary()]);
    } catch (error) {
      appState.update((state) => ({
        ...state,
        referenceError:
          error instanceof Error ? error.message : "Failed to update the reference library."
      }));
    }
  }
};

export const selectedProfile = derived(appState, ($appState) =>
  $appState.profiles.find((profile) => profile.id === $appState.selectedProfileId)
);
