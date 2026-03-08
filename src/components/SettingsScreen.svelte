<script lang="ts">
  import type {
    CacheSummaryDto,
    OnboardingStatusDto,
    ProfilesStorageSummaryDto,
    ProtonRuntime,
    StorageLocationsDto,
    StorageMigrationStatusDto
  } from "../lib/types";
  import Icon from "./Icon.svelte";
  import SimpleConfirmModal from "./SimpleConfirmModal.svelte";

  export let warningPrefs: {
    red: boolean;
    broken: boolean;
    installWithoutDependencies: boolean;
    uninstallWithDependants: boolean;
    importProfilePack: boolean;
    conserveWhileGameRunning: boolean;
  };
  export let cacheSummary: CacheSummaryDto | undefined;
  export let profilesStorageSummary: ProfilesStorageSummaryDto | undefined;
  export let storageLocations: StorageLocationsDto | undefined;
  export let storageMigration: StorageMigrationStatusDto | null = null;
  export let isLoadingProfilesStorageSummary = false;
  export let onboardingStatus: OnboardingStatusDto | undefined;
  export let protonRuntimes: ProtonRuntime[] = [];
  export let selectedProtonRuntimeId: string | null = null;
  export let isLoadingProtonRuntimes = false;
  export let settingsError: string | null = null;
  export let onWarningPrefChange: (
    kind:
      | "red"
      | "broken"
      | "installWithoutDependencies"
      | "uninstallWithDependants"
      | "importProfilePack"
      | "conserveWhileGameRunning",
    enabled: boolean
  ) => void | Promise<void>;
  export let onOpenCacheFolder: () => void | Promise<void>;
  export let onMoveCacheLocation: () => void | Promise<void>;
  export let onOpenProfilesFolder: () => void | Promise<void>;
  export let onMoveProfilesLocation: () => void | Promise<void>;
  export let onOpenActiveProfileFolder: () => void | Promise<void>;
  export let onOpenMemoryDiagnostics: () => void | Promise<void>;
  export let onClearCache: () => void | Promise<void>;
  export let onClearUnreferencedCache: () => void | Promise<void>;
  export let onResetAllData: () => void | Promise<void>;
  export let onRunOnboardingAgain: () => void | Promise<void>;
  export let onSelectProtonRuntime: (runtimeId: string) => void | Promise<void>;
  let showClearCacheConfirm = false;
  let showResetConfirm = false;

  function formatDiskSpace(value = 0) {
    const bytes = Math.max(0, Math.trunc(value));
    const units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let scaled = bytes;
    let unitIndex = 0;

    while (scaled >= 1024 && unitIndex < units.length - 1) {
      scaled /= 1024;
      unitIndex += 1;
    }

    const maximumFractionDigits = unitIndex === 0 ? 0 : scaled >= 100 ? 0 : scaled >= 10 ? 1 : 2;

    return `${scaled.toLocaleString(undefined, {
      maximumFractionDigits,
      minimumFractionDigits: 0
    })} ${units[unitIndex]}`;
  }

  async function runClearCache() {
    showClearCacheConfirm = false;
    await onClearCache();
  }

  async function runResetAllData() {
    showResetConfirm = false;
    await onResetAllData();
  }

  function handleProtonRuntimeChange(event: Event) {
    const value = (event.currentTarget as HTMLSelectElement).value;
    void onSelectProtonRuntime(value);
  }

  function formatTimestamp(value?: string) {
    if (!value) {
      return "Never";
    }

    const date = new Date(value);
    if (Number.isNaN(date.getTime())) {
      return value;
    }

    return date.toLocaleString();
  }
</script>

<section class="screen-stack settings-screen">
  <section class="panel compact-panel">
    <div class="compact-heading compact-heading-left">
      <Icon label="Settings" name="settings" />
      <h2>Global settings</h2>
    </div>

    <div class="settings-section">
      <div class="settings-subheading">
        <h3>Warn options</h3>
      </div>

      <div class="preference-list">
        <div class="switch-row">
          <div>
            <strong>Warn on red downloads</strong>
            <p>Ask before installing versions from the red zone.</p>
          </div>
          <button
            aria-pressed={warningPrefs.red}
            class="ghost-button icon-button toggle-icon-button"
            type="button"
            on:click={() => onWarningPrefChange("red", !warningPrefs.red)}
          >
            <Icon label={warningPrefs.red ? "Enabled" : "Disabled"} name={warningPrefs.red ? "check" : "circle"} />
            <span>{warningPrefs.red ? "On" : "Off"}</span>
          </button>
        </div>

        <div class="switch-row">
          <div>
            <strong>Warn on broken downloads</strong>
            <p>Ask before installing versions marked broken locally.</p>
          </div>
          <button
            aria-pressed={warningPrefs.broken}
            class="ghost-button icon-button toggle-icon-button"
            type="button"
            on:click={() => onWarningPrefChange("broken", !warningPrefs.broken)}
          >
            <Icon label={warningPrefs.broken ? "Enabled" : "Disabled"} name={warningPrefs.broken ? "check" : "circle"} />
            <span>{warningPrefs.broken ? "On" : "Off"}</span>
          </button>
        </div>

        <div class="switch-row">
          <div>
            <strong>Warn on install without dependencies</strong>
            <p>Ask for confirmation before installing while skipping dependency installs.</p>
          </div>
          <button
            aria-pressed={warningPrefs.installWithoutDependencies}
            class="ghost-button icon-button toggle-icon-button"
            type="button"
            on:click={() =>
              onWarningPrefChange(
                "installWithoutDependencies",
                !warningPrefs.installWithoutDependencies
              )}
          >
            <Icon
              label={warningPrefs.installWithoutDependencies ? "Enabled" : "Disabled"}
              name={warningPrefs.installWithoutDependencies ? "check" : "circle"}
            />
            <span>{warningPrefs.installWithoutDependencies ? "On" : "Off"}</span>
          </button>
        </div>

        <div class="switch-row">
          <div>
            <strong>Warn on uninstall with dependants</strong>
            <p>Ask before uninstalling mods that other installed mods depend on.</p>
          </div>
          <button
            aria-pressed={warningPrefs.uninstallWithDependants}
            class="ghost-button icon-button toggle-icon-button"
            type="button"
            on:click={() =>
              onWarningPrefChange(
                "uninstallWithDependants",
                !warningPrefs.uninstallWithDependants
              )}
          >
            <Icon
              label={warningPrefs.uninstallWithDependants ? "Enabled" : "Disabled"}
              name={warningPrefs.uninstallWithDependants ? "check" : "circle"}
            />
            <span>{warningPrefs.uninstallWithDependants ? "On" : "Off"}</span>
          </button>
        </div>

        <div class="switch-row">
          <div>
            <strong>Warn on profile import</strong>
            <p>Show a confirmation preview with mod list before importing a .49pack file.</p>
          </div>
          <button
            aria-pressed={warningPrefs.importProfilePack}
            class="ghost-button icon-button toggle-icon-button"
            type="button"
            on:click={() =>
              onWarningPrefChange(
                "importProfilePack",
                !warningPrefs.importProfilePack
              )}
          >
            <Icon
              label={warningPrefs.importProfilePack ? "Enabled" : "Disabled"}
              name={warningPrefs.importProfilePack ? "check" : "circle"}
            />
            <span>{warningPrefs.importProfilePack ? "On" : "Off"}</span>
          </button>
        </div>
      </div>
    </div>

    <div class="settings-section">
      <div class="settings-subheading">
        <h3>Performance</h3>
      </div>

      <div class="preference-list">
        <div class="switch-row">
          <div>
            <strong>Conserve resources while game is running</strong>
            <p>Pause background polling and non-essential refresh activity while Lethal Company is open.</p>
          </div>
          <button
            aria-pressed={warningPrefs.conserveWhileGameRunning}
            class="ghost-button icon-button toggle-icon-button"
            type="button"
            on:click={() =>
              onWarningPrefChange(
                "conserveWhileGameRunning",
                !warningPrefs.conserveWhileGameRunning
              )}
          >
            <Icon
              label={warningPrefs.conserveWhileGameRunning ? "Enabled" : "Disabled"}
              name={warningPrefs.conserveWhileGameRunning ? "check" : "circle"}
            />
            <span>{warningPrefs.conserveWhileGameRunning ? "On" : "Off"}</span>
          </button>
        </div>

        <div class="switch-row">
          <div>
            <strong>View RAM usage</strong>
            <p>Open a live process snapshot showing how much memory 49modman is using.</p>
          </div>
          <button class="ghost-button icon-button" type="button" on:click={onOpenMemoryDiagnostics}>
            <Icon label="Open RAM diagnostics" name="details" />
            <span>Open</span>
          </button>
        </div>
      </div>
    </div>

    <div class="settings-section">
      <div class="settings-subheading">
        <h3>Onboarding</h3>
      </div>

      <div class="preference-list">
        <div class="switch-row">
          <div>
            <strong>Run onboarding again</strong>
            <p>
              Validate your v49 install again without enabling the first-run lock.
              Last completed: {formatTimestamp(onboardingStatus?.completedAt)}
            </p>
          </div>
          <button class="ghost-button icon-button" type="button" on:click={onRunOnboardingAgain} disabled={storageMigration?.isActive}>
            <Icon label="Run onboarding again" name="refresh" />
            <span>Run</span>
          </button>
        </div>
      </div>
    </div>

    <div class="settings-section">
      <div class="settings-subheading">
        <h3>Cache</h3>
      </div>

      <div class="preference-list">
        <div class="switch-row">
          <div>
            <strong>Cached archives</strong>
            <p>{cacheSummary ? `${cacheSummary.archiveCount} archives · ${formatDiskSpace(cacheSummary.totalBytes)}` : "Loading cache summary..."}</p>
          </div>
        </div>

        <div class="switch-row">
          <div>
            <strong>Cache location</strong>
            <p>{storageLocations ? storageLocations.cacheDir : "Loading cache path..."}</p>
          </div>
          <button class="ghost-button icon-button" type="button" on:click={onMoveCacheLocation} disabled={storageMigration?.isActive}>
            <Icon label="Move cache location" name="folder" />
            <span>Move</span>
          </button>
        </div>

        <div class="switch-row">
          <div>
            <strong>Open cache folder</strong>
            <p>View the shared archive cache in the system file explorer.</p>
          </div>
          <button class="ghost-button icon-button" type="button" on:click={onOpenCacheFolder} disabled={storageMigration?.isActive}>
            <Icon label="Open cache folder" name="folder" />
            <span>Open</span>
          </button>
        </div>

        <div class="switch-row danger-row">
          <div>
            <strong>Clear cache</strong>
            <p>Remove all cached mod archives from local storage.</p>
          </div>
        <button
          class="ghost-button danger-outline"
          disabled={cacheSummary?.hasActiveDownloads || storageMigration?.isActive}
          type="button"
          on:click={() => (showClearCacheConfirm = true)}
        >
          Clear cache
        </button>
        </div>

        <div class="switch-row danger-row">
          <div>
            <strong>Clear unreferenced cache</strong>
            <p>Remove cached versions not installed in any profile. Disabled installed mods are kept.</p>
          </div>
          <button
            class="ghost-button danger-outline"
            disabled={cacheSummary?.hasActiveDownloads || storageMigration?.isActive}
            type="button"
            on:click={onClearUnreferencedCache}
          >
            Review and clear
          </button>
        </div>
      </div>
    </div>

    {#if isLoadingProtonRuntimes || protonRuntimes.length > 0 || selectedProtonRuntimeId}
      <div class="settings-section">
        <div class="settings-subheading">
          <h3>Launch (Linux)</h3>
        </div>

        <div class="preference-list">
          <div class="switch-row">
            <div>
              <strong>Preferred Proton runtime</strong>
              <p>Used for Direct launch mode. Steam launch mode still uses Steam compatibility settings.</p>
            </div>
            <label class="settings-select">
              <select
                disabled={isLoadingProtonRuntimes || protonRuntimes.length === 0}
                value={selectedProtonRuntimeId ?? ""}
                on:change={handleProtonRuntimeChange}
              >
                <option value="" disabled={true}>
                  {#if isLoadingProtonRuntimes}
                    Loading runtimes...
                  {:else if protonRuntimes.length === 0}
                    No runtimes found
                  {:else}
                    Select runtime
                  {/if}
                </option>
                {#each protonRuntimes as runtime}
                  <option value={runtime.id} disabled={!runtime.isValid}>
                    {runtime.displayName}{runtime.isValid ? "" : " (invalid)"}
                  </option>
                {/each}
              </select>
            </label>
          </div>
        </div>
      </div>
    {/if}

    <div class="settings-section">
      <div class="settings-subheading">
        <h3>Profiles</h3>
      </div>

      <div class="preference-list">
        <div class="switch-row">
          <div>
            <strong>Profiles location</strong>
            <p>{storageLocations ? storageLocations.profilesDir : "Loading profiles path..."}</p>
          </div>
          <button class="ghost-button icon-button" type="button" on:click={onMoveProfilesLocation} disabled={storageMigration?.isActive}>
            <Icon label="Move profiles location" name="folder" />
            <span>Move</span>
          </button>
        </div>

        <div class="switch-row">
          <div>
            <strong>Profile storage</strong>
            <p>
              {#if isLoadingProfilesStorageSummary}
                Loading profile summary...
              {:else if profilesStorageSummary}
                {profilesStorageSummary.profileCount} profiles · {formatDiskSpace(profilesStorageSummary.profilesTotalBytes)} total · {formatDiskSpace(profilesStorageSummary.activeProfileBytes)} active
              {:else}
                Profile summary unavailable.
              {/if}
            </p>
          </div>
        </div>

        <div class="switch-row">
          <div>
            <strong>Open profiles folder</strong>
            <p>Open the root directory that contains all local profile folders.</p>
          </div>
          <button class="ghost-button icon-button" type="button" on:click={onOpenProfilesFolder} disabled={storageMigration?.isActive}>
            <Icon label="Open profiles folder" name="folder" />
            <span>Open</span>
          </button>
        </div>

        <div class="switch-row">
          <div>
            <strong>Open active profile folder</strong>
            <p>Open the currently active profile folder and its manifest files.</p>
          </div>
          <button class="ghost-button icon-button" type="button" on:click={onOpenActiveProfileFolder} disabled={storageMigration?.isActive}>
            <Icon label="Open active profile folder" name="folder" />
            <span>Open</span>
          </button>
        </div>
      </div>
    </div>

    <div class="settings-section danger-zone">
      <div class="settings-subheading">
        <h3>Danger zone</h3>
      </div>

      <div class="switch-row danger-row">
        <div>
          <strong>Reset all data</strong>
          <p>Clear profiles, settings, local overrides, cached catalog metadata, and cached archives.</p>
        </div>
        <button
          class="ghost-button danger-outline"
          type="button"
          disabled={storageMigration?.isActive}
          on:click={() => (showResetConfirm = true)}
        >
          Reset
        </button>
      </div>
    </div>

    {#if settingsError}
      <p class="warning-copy danger settings-error">{settingsError}</p>
    {/if}
  </section>
</section>

{#if showClearCacheConfirm}
  <SimpleConfirmModal
    title="Clear all cached archives?"
    description="This removes downloaded versions from local storage, but it does not delete profiles."
    confirmLabel="Clear cache"
    confirmIcon="trash"
    isDanger={true}
    onCancel={() => (showClearCacheConfirm = false)}
    onConfirm={runClearCache}
  />
{/if}

{#if showResetConfirm}
  <SimpleConfirmModal
    title="Reset all app data?"
    description="This clears profiles, settings, cached metadata, and cached archives."
    confirmLabel="Reset all data"
    confirmIcon="trash"
    isDanger={true}
    onCancel={() => (showResetConfirm = false)}
    onConfirm={runResetAllData}
  />
{/if}
