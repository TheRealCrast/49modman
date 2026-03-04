import {
  getCatalogSummaryMock,
  getPackageDetailMock,
  getWarningPrefsMock,
  listReferenceRowsMock,
  searchPackagesMock,
  setReferenceStateMock,
  setWarningPreferenceMock,
  syncCatalogMock
} from "./mock-backend";

declare global {
  interface Window {
    __TAURI__?: {
      core?: {
        invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
      };
    };
  }
}

type CommandMap = {
  sync_catalog: typeof syncCatalogMock;
  get_catalog_summary: typeof getCatalogSummaryMock;
  search_packages: typeof searchPackagesMock;
  get_package_detail: typeof getPackageDetailMock;
  list_reference_rows: typeof listReferenceRowsMock;
  set_reference_state: typeof setReferenceStateMock;
  get_warning_prefs: typeof getWarningPrefsMock;
  set_warning_preference: typeof setWarningPreferenceMock;
};

const mockCommands: CommandMap = {
  sync_catalog: syncCatalogMock,
  get_catalog_summary: getCatalogSummaryMock,
  search_packages: searchPackagesMock,
  get_package_detail: getPackageDetailMock,
  list_reference_rows: listReferenceRowsMock,
  set_reference_state: setReferenceStateMock,
  get_warning_prefs: getWarningPrefsMock,
  set_warning_preference: setWarningPreferenceMock
};

function tauriInvoke() {
  return window.__TAURI__?.core?.invoke;
}

export async function invokeCommand<T>(
  command: keyof CommandMap,
  args?: Record<string, unknown>
): Promise<T> {
  const invoke = tauriInvoke();

  if (invoke) {
    return invoke<T>(command, args);
  }

  const handler = mockCommands[command] as (...values: unknown[]) => Promise<T>;

  if (!handler) {
    throw new Error(`Unsupported mock command: ${command}`);
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

  if (args && "kind" in args && "enabled" in args) {
    return handler(args.kind, args.enabled);
  }

  return handler();
}
