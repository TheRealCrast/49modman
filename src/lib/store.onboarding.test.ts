import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api/cache", () => ({
  clearCache: vi.fn(async () => ({
    archiveCount: 0,
    totalBytes: 0,
    cachePath: "/cache",
    hasActiveDownloads: false
  })),
  clearCacheUnreferenced: vi.fn(async () => ({
    archiveCount: 0,
    totalBytes: 0,
    cachePath: "/cache",
    hasActiveDownloads: false
  })),
  getCacheSummary: vi.fn(async () => ({
    archiveCount: 0,
    totalBytes: 0,
    cachePath: "/cache",
    hasActiveDownloads: false
  })),
  openCacheFolder: vi.fn(async () => undefined),
  previewClearCacheUnreferenced: vi.fn(async () => ({
    removableCount: 0,
    removableBytes: 0,
    candidates: []
  })),
  queueInstallToCache: vi.fn(async () => ({ taskId: "task-1" }))
}));

vi.mock("./api/catalog", () => ({
  getCatalogSummary: vi.fn(async () => ({
    hasCatalog: true,
    packageCount: 0,
    versionCount: 0,
    lastSyncLabel: "Never"
  })),
  getPackageDetail: vi.fn(async () => null),
  searchPackages: vi.fn(async () => ({
    items: [],
    nextCursor: null,
    hasMore: false,
    pageSize: 40
  })),
  syncCatalog: vi.fn(async () => ({
    outcome: "skipped",
    packageCount: 0,
    versionCount: 0,
    syncedAt: null,
    message: "Skipped"
  }))
}));

vi.mock("./api/dependencies", () => ({
  getVersionDependencies: vi.fn(async () => ({
    rootPackageId: "root",
    rootPackageName: "Root",
    rootVersionId: "v1",
    rootVersionNumber: "1.0.0",
    summary: { direct: [], transitive: [], unresolved: [] },
    treeItems: []
  })),
  warmDependencyIndex: vi.fn(async () => undefined)
}));

vi.mock("./api/downloads", () => ({
  listActiveDownloads: vi.fn(async () => [])
}));

vi.mock("./api/launch", () => ({
  getLaunchRuntimeStatus: vi.fn(async () => ({ isGameRunning: false })),
  getMemoryDiagnostics: vi.fn(async () => ({
    capturedAt: new Date().toISOString(),
    platform: "test",
    processes: [],
    totals: { rssBytes: 0 },
    notes: []
  })),
  launchProfile: vi.fn(async () => ({ ok: true, code: "OK", message: "ok" })),
  launchVanilla: vi.fn(async () => ({ ok: true, code: "OK", message: "ok" })),
  listProtonRuntimes: vi.fn(async () => []),
  pickGameInstallFolder: vi.fn(async () => null),
  repairActivation: vi.fn(async () => ({
    ok: true,
    code: "OK",
    message: "ok",
    manifestPath: null,
    gamePath: null,
    removedFileCount: 0,
    removedDirCount: 0,
    missingEntryCount: 0,
    remainingEntryCount: 0
  })),
  scanSteamInstallations: vi.fn(async () => ({
    steamRootPaths: [],
    libraryPaths: [],
    gamePaths: [],
    selectedGamePath: null
  })),
  setPreferredProtonRuntime: vi.fn(async () => undefined),
  trimResourceSaverMemory: vi.fn(async () => ({
    ok: true,
    code: "OK",
    message: "ok"
  })),
  validateV49Install: vi.fn(async () => ({
    ok: true,
    code: "OK",
    message: "ok",
    resolvedGamePath: "/games/Lethal Company",
    resolvedFrom: "override",
    selectedProfileId: null,
    checks: []
  }))
}));

vi.mock("./api/profiles", () => ({
  createProfile: vi.fn(async () => ({
    id: "default",
    name: "Default",
    notes: "",
    gamePath: "",
    lastPlayed: null,
    launchModeDefault: "steam",
    isBuiltinDefault: true,
    installedMods: []
  })),
  deleteProfile: vi.fn(async () => ({ deletedId: "default", nextActiveProfileId: null })),
  exportProfilePack: vi.fn(async () => ({ cancelled: true, payloadMode: "compact", embeddedModCount: 0, referencedModCount: 0, hasLegacyRuntimePluginsPayload: false })),
  getActiveProfile: vi.fn(async () => null),
  getUninstallDependants: vi.fn(async () => []),
  importProfileModZip: vi.fn(async () => ({ cancelled: true, addedToCache: false })),
  importProfilePackFromPath: vi.fn(async () => ({
    cancelled: true,
    payloadMode: "compact",
    embeddedModCount: 0,
    referencedModCount: 0,
    hasLegacyRuntimePluginsPayload: false
  })),
  previewImportProfilePack: vi.fn(async () => ({
    cancelled: true,
    payloadMode: "compact",
    embeddedModCount: 0,
    referencedModCount: 0,
    hasLegacyRuntimePluginsPayload: false,
    mods: []
  })),
  getProfilesStorageSummary: vi.fn(async () => ({
    profileCount: 1,
    profilesTotalBytes: 0,
    activeProfileBytes: 0
  })),
  listProfiles: vi.fn(async () => []),
  openActiveProfileFolder: vi.fn(async () => undefined),
  openProfilesFolder: vi.fn(async () => undefined),
  previewExportProfilePack: vi.fn(async () => ({
    profileId: "default",
    profileName: "Default",
    modCount: 0,
    unavailableMods: []
  })),
  resetAllData: vi.fn(async () => undefined),
  setActiveProfile: vi.fn(async () => null),
  setInstalledModEnabled: vi.fn(async () => ({
    id: "default",
    name: "Default",
    notes: "",
    gamePath: "",
    lastPlayed: null,
    launchModeDefault: "steam",
    isBuiltinDefault: true,
    installedMods: []
  })),
  uninstallInstalledMod: vi.fn(async () => ({
    id: "default",
    name: "Default",
    notes: "",
    gamePath: "",
    lastPlayed: null,
    launchModeDefault: "steam",
    isBuiltinDefault: true,
    installedMods: []
  })),
  updateProfile: vi.fn(async () => ({
    id: "default",
    name: "Default",
    notes: "",
    gamePath: "",
    lastPlayed: null,
    launchModeDefault: "steam",
    isBuiltinDefault: true,
    installedMods: []
  }))
}));

vi.mock("./api/reference", () => ({
  listReferenceRows: vi.fn(async () => ({
    items: [],
    nextCursor: null,
    hasMore: false,
    pageSize: 50
  })),
  setReferenceState: vi.fn(async () => ({
    packageId: "pkg",
    packageName: "pkg",
    versionId: "v1",
    versionNumber: "1.0.0",
    publishedAt: new Date().toISOString(),
    baseZone: "green",
    effectiveStatus: "green"
  }))
}));

vi.mock("./api/settings", () => ({
  completeOnboarding: vi.fn(async () => ({
    completed: true,
    completedAt: new Date().toISOString(),
    lastValidatedGamePath: "/games/Lethal Company"
  })),
  getOnboardingStatus: vi.fn(async () => ({
    completed: false
  })),
  getStorageLocations: vi.fn(async () => ({
    cacheDir: "/cache",
    profilesDir: "/profiles"
  })),
  getStorageMigrationStatus: vi.fn(async () => ({
    phase: "idle",
    message: "Idle",
    bytesCopied: 0,
    totalBytes: 0,
    percentComplete: 0,
    isActive: false
  })),
  getWarningPrefs: vi.fn(async () => ({
    red: true,
    broken: true,
    installWithoutDependencies: true,
    uninstallWithDependants: true,
    importProfilePack: true,
    conserveWhileGameRunning: false
  })),
  pickStorageFolder: vi.fn(async () => null),
  setWarningPreference: vi.fn(async () => ({
    red: true,
    broken: true,
    installWithoutDependencies: true,
    uninstallWithDependants: true,
    importProfilePack: true,
    conserveWhileGameRunning: false
  })),
  startStorageMigration: vi.fn(async () => ({
    phase: "idle",
    message: "Idle",
    bytesCopied: 0,
    totalBytes: 0,
    percentComplete: 0,
    isActive: false
  }))
}));

vi.mock("./api/system", () => ({
  openExternalUrl: vi.fn(async () => undefined)
}));

describe("store onboarding bootstrap branches", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  it("enters required onboarding mode when onboarding is incomplete", async () => {
    const settingsApi = await import("./api/settings");
    const launchApi = await import("./api/launch");
    const catalogApi = await import("./api/catalog");

    vi.mocked(settingsApi.getOnboardingStatus).mockResolvedValue({
      completed: false
    });
    vi.mocked(launchApi.scanSteamInstallations).mockResolvedValue({
      steamRootPaths: ["/steam"],
      libraryPaths: ["/steam/steamapps"],
      gamePaths: ["/steam/steamapps/common/Lethal Company"],
      selectedGamePath: "/steam/steamapps/common/Lethal Company"
    });

    const { actions, appState } = await import("./store");
    await actions.bootstrap();

    const state = get(appState);
    expect(state.onboardingRequired).toBe(true);
    expect(state.onboardingMode).toBe("required");
    expect(state.view).toBe("onboarding");
    expect(state.onboardingPathDraft).toBe("/steam/steamapps/common/Lethal Company");
    expect(vi.mocked(catalogApi.getCatalogSummary)).not.toHaveBeenCalled();
  });

  it("does not re-enter onboarding when completion is already true", async () => {
    const settingsApi = await import("./api/settings");
    const launchApi = await import("./api/launch");
    const catalogApi = await import("./api/catalog");

    vi.mocked(settingsApi.getOnboardingStatus).mockResolvedValue({
      completed: true,
      completedAt: "2026-03-08T00:00:00Z",
      lastValidatedGamePath: "/games/Lethal Company"
    });
    vi.mocked(catalogApi.getCatalogSummary).mockRejectedValue(
      new Error("Stop bootstrap after onboarding branch assertion.")
    );

    const { actions, appState } = await import("./store");
    await actions.bootstrap();

    const state = get(appState);
    expect(state.onboardingRequired).toBe(false);
    expect(state.onboardingMode).toBeNull();
    expect(state.view).toBe("browse");
    expect(vi.mocked(catalogApi.getCatalogSummary)).toHaveBeenCalledTimes(1);
  });

  it("blocks leaving onboarding only in required mode", async () => {
    const { actions, appState } = await import("./store");

    appState.update((state) => ({
      ...state,
      onboardingRequired: true,
      onboardingMode: "required",
      view: "onboarding"
    }));
    actions.setView("settings");
    expect(get(appState).view).toBe("onboarding");

    appState.update((state) => ({
      ...state,
      onboardingRequired: false,
      onboardingMode: "manual",
      view: "onboarding"
    }));
    actions.setView("settings");
    expect(get(appState).view).toBe("settings");
  });
});
