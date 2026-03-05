import {
  clearCacheMock,
  createProfileMock,
  deleteProfileMock,
  getVersionDependenciesMock,
  warmDependencyIndexMock,
  getCacheSummaryMock,
  getActiveProfileMock,
  getCatalogSummaryMock,
  getTaskMock,
  getPackageDetailMock,
  getProfileDetailMock,
  getProfilesStorageSummaryMock,
  getWarningPrefsMock,
  listActiveDownloadsMock,
  listProfilesMock,
  listReferenceRowsMock,
  openCacheFolderMock,
  openActiveProfileFolderMock,
  openProfilesFolderMock,
  setInstalledModEnabledMock,
  queueInstallToCacheMock,
  resetAllDataMock,
  searchPackagesMock,
  setActiveProfileMock,
  uninstallInstalledModMock,
  updateProfileMock,
  setReferenceStateMock,
  setWarningPreferenceMock,
  syncCatalogMock
} from "./mock-backend";
import { getRuntimeKind, isTauriRuntime } from "../runtime";

type CommandMap = {
  sync_catalog: typeof syncCatalogMock;
  get_catalog_summary: typeof getCatalogSummaryMock;
  search_packages: typeof searchPackagesMock;
  get_package_detail: typeof getPackageDetailMock;
  get_version_dependencies: typeof getVersionDependenciesMock;
  warm_dependency_index: typeof warmDependencyIndexMock;
  list_profiles: typeof listProfilesMock;
  get_active_profile: typeof getActiveProfileMock;
  set_active_profile: typeof setActiveProfileMock;
  create_profile: typeof createProfileMock;
  update_profile: typeof updateProfileMock;
  delete_profile: typeof deleteProfileMock;
  get_profile_detail: typeof getProfileDetailMock;
  reset_all_data: typeof resetAllDataMock;
  open_profiles_folder: typeof openProfilesFolderMock;
  open_active_profile_folder: typeof openActiveProfileFolderMock;
  get_profiles_storage_summary: typeof getProfilesStorageSummaryMock;
  set_installed_mod_enabled: typeof setInstalledModEnabledMock;
  uninstall_installed_mod: typeof uninstallInstalledModMock;
  list_reference_rows: typeof listReferenceRowsMock;
  set_reference_state: typeof setReferenceStateMock;
  get_warning_prefs: typeof getWarningPrefsMock;
  set_warning_preference: typeof setWarningPreferenceMock;
  queue_install_to_cache: typeof queueInstallToCacheMock;
  get_cache_summary: typeof getCacheSummaryMock;
  open_cache_folder: typeof openCacheFolderMock;
  clear_cache: typeof clearCacheMock;
  list_active_downloads: typeof listActiveDownloadsMock;
  get_task: typeof getTaskMock;
  open_external_url: (url: string) => Promise<void>;
};

const mockCommands: CommandMap = {
  sync_catalog: syncCatalogMock,
  get_catalog_summary: getCatalogSummaryMock,
  search_packages: searchPackagesMock,
  get_package_detail: getPackageDetailMock,
  get_version_dependencies: getVersionDependenciesMock,
  warm_dependency_index: warmDependencyIndexMock,
  list_profiles: listProfilesMock,
  get_active_profile: getActiveProfileMock,
  set_active_profile: setActiveProfileMock,
  create_profile: createProfileMock,
  update_profile: updateProfileMock,
  delete_profile: deleteProfileMock,
  get_profile_detail: getProfileDetailMock,
  reset_all_data: resetAllDataMock,
  open_profiles_folder: openProfilesFolderMock,
  open_active_profile_folder: openActiveProfileFolderMock,
  get_profiles_storage_summary: getProfilesStorageSummaryMock,
  set_installed_mod_enabled: setInstalledModEnabledMock,
  uninstall_installed_mod: uninstallInstalledModMock,
  list_reference_rows: listReferenceRowsMock,
  set_reference_state: setReferenceStateMock,
  get_warning_prefs: getWarningPrefsMock,
  set_warning_preference: setWarningPreferenceMock,
  queue_install_to_cache: queueInstallToCacheMock,
  get_cache_summary: getCacheSummaryMock,
  open_cache_folder: openCacheFolderMock,
  clear_cache: clearCacheMock,
  list_active_downloads: listActiveDownloadsMock,
  get_task: getTaskMock,
  open_external_url: async () => {}
};

function tauriInvoke() {
  return window.__TAURI__?.core?.invoke;
}

export { getRuntimeKind };

export async function invokeCommand<T>(
  command: keyof CommandMap,
  args?: Record<string, unknown>
): Promise<T> {
  if (isTauriRuntime()) {
    const invoke = tauriInvoke();

    if (!invoke) {
      throw new Error(`Tauri runtime is available but invoke() is missing for command ${command}.`);
    }

    return invoke<T>(command, args);
  }

  const handler = mockCommands[command] as (...values: unknown[]) => Promise<T>;

  if (!handler) {
    throw new Error(`Unsupported mock command: ${command}`);
  }

  if (args && "input" in args) {
    return handler(args.input);
  }

  if (args && "input" in args) {
    return handler(args.input);
  }

  if (args && "query" in args) {
    return handler(args.query);
  }

  if (args && "packageId" in args) {
    return handler(args.packageId);
  }

  if (args && "profileId" in args) {
    return handler(args.profileId);
  }

  if (args && "url" in args) {
    return handler(args.url);
  }

  if (args && "taskId" in args) {
    return handler(args.taskId);
  }

  if (args && "kind" in args && "enabled" in args) {
    return handler(args.kind, args.enabled);
  }

  return handler();
}
