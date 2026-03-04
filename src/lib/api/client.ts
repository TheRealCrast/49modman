import {
  createProfileMock,
  deleteProfileMock,
  getActiveProfileMock,
  getCatalogSummaryMock,
  getPackageDetailMock,
  getProfileDetailMock,
  getWarningPrefsMock,
  listProfilesMock,
  listReferenceRowsMock,
  resetAllDataMock,
  searchPackagesMock,
  setActiveProfileMock,
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
  list_profiles: typeof listProfilesMock;
  get_active_profile: typeof getActiveProfileMock;
  set_active_profile: typeof setActiveProfileMock;
  create_profile: typeof createProfileMock;
  update_profile: typeof updateProfileMock;
  delete_profile: typeof deleteProfileMock;
  get_profile_detail: typeof getProfileDetailMock;
  reset_all_data: typeof resetAllDataMock;
  list_reference_rows: typeof listReferenceRowsMock;
  set_reference_state: typeof setReferenceStateMock;
  get_warning_prefs: typeof getWarningPrefsMock;
  set_warning_preference: typeof setWarningPreferenceMock;
  open_external_url: (url: string) => Promise<void>;
};

const mockCommands: CommandMap = {
  sync_catalog: syncCatalogMock,
  get_catalog_summary: getCatalogSummaryMock,
  search_packages: searchPackagesMock,
  get_package_detail: getPackageDetailMock,
  list_profiles: listProfilesMock,
  get_active_profile: getActiveProfileMock,
  set_active_profile: setActiveProfileMock,
  create_profile: createProfileMock,
  update_profile: updateProfileMock,
  delete_profile: deleteProfileMock,
  get_profile_detail: getProfileDetailMock,
  reset_all_data: resetAllDataMock,
  list_reference_rows: listReferenceRowsMock,
  set_reference_state: setReferenceStateMock,
  get_warning_prefs: getWarningPrefsMock,
  set_warning_preference: setWarningPreferenceMock,
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

  if (args && "kind" in args && "enabled" in args) {
    return handler(args.kind, args.enabled);
  }

  return handler();
}
