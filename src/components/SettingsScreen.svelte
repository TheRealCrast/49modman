<script lang="ts">
  import type { CacheSummaryDto, ProfilesStorageSummaryDto } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let warningPrefs: {
    red: boolean;
    broken: boolean;
  };
  export let cacheSummary: CacheSummaryDto | undefined;
  export let profilesStorageSummary: ProfilesStorageSummaryDto | undefined;
  export let isLoadingProfilesStorageSummary = false;
  export let settingsError: string | null = null;
  export let onWarningPrefChange: (kind: "red" | "broken", enabled: boolean) => void | Promise<void>;
  export let onOpenCacheFolder: () => void | Promise<void>;
  export let onOpenProfilesFolder: () => void | Promise<void>;
  export let onOpenActiveProfileFolder: () => void | Promise<void>;
  export let onClearCache: () => void | Promise<void>;
  export let onResetAllData: () => void | Promise<void>;

  function formatBytes(value = 0) {
    if (value < 1024) {
      return `${value} B`;
    }

    if (value < 1024 * 1024) {
      return `${(value / 1024).toFixed(0)} KB`;
    }

    return `${(value / (1024 * 1024)).toFixed(1)} MB`;
  }

  async function confirmClearCache() {
    if (
      !window.confirm(
        "Clear all cached mod archives? This will remove downloaded versions from local storage, but it will not delete profiles."
      )
    ) {
      return;
    }

    await onClearCache();
  }

  async function confirmReset() {
    if (!window.confirm("Reset all app data and return to a clean first-launch state?")) {
      return;
    }

    await onResetAllData();
  }
</script>

<section class="screen-stack">
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
            <p>{cacheSummary ? `${cacheSummary.archiveCount} archives · ${formatBytes(cacheSummary.totalBytes)}` : "Loading cache summary..."}</p>
          </div>
        </div>

        <div class="switch-row">
          <div>
            <strong>Open cache folder</strong>
            <p>View the shared archive cache in the system file explorer.</p>
          </div>
          <button class="ghost-button icon-button" type="button" on:click={onOpenCacheFolder}>
            <Icon label="Open cache folder" name="folder" />
            <span>Open</span>
          </button>
        </div>
      </div>
    </div>

    <div class="settings-section">
      <div class="settings-subheading">
        <h3>Profiles</h3>
      </div>

      <div class="preference-list">
        <div class="switch-row">
          <div>
            <strong>Profile storage</strong>
            <p>
              {#if isLoadingProfilesStorageSummary}
                Loading profile summary...
              {:else if profilesStorageSummary}
                {profilesStorageSummary.profileCount} profiles · {formatBytes(profilesStorageSummary.profilesTotalBytes)} total · {formatBytes(profilesStorageSummary.activeProfileBytes)} active
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
          <button class="ghost-button icon-button" type="button" on:click={onOpenProfilesFolder}>
            <Icon label="Open profiles folder" name="folder" />
            <span>Open</span>
          </button>
        </div>

        <div class="switch-row">
          <div>
            <strong>Open active profile folder</strong>
            <p>Open the currently active profile folder and its manifest files.</p>
          </div>
          <button class="ghost-button icon-button" type="button" on:click={onOpenActiveProfileFolder}>
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
          <strong>Clear cache</strong>
          <p>Remove all cached mod archives from local storage.</p>
        </div>
        <button
          class="ghost-button danger-outline"
          disabled={cacheSummary?.hasActiveDownloads}
          type="button"
          on:click={confirmClearCache}
        >
          Clear cache
        </button>
      </div>

      <div class="switch-row danger-row">
        <div>
          <strong>Reset all data</strong>
          <p>Clear profiles, settings, local overrides, cached catalog metadata, and cached archives.</p>
        </div>
        <button class="ghost-button danger-outline" type="button" on:click={confirmReset}>
          Reset
        </button>
      </div>
    </div>

    {#if settingsError}
      <p class="warning-copy danger settings-error">{settingsError}</p>
    {/if}
  </section>
</section>
