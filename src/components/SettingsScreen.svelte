<script lang="ts">
  import Icon from "./Icon.svelte";

  export let warningPrefs: {
    red: boolean;
    broken: boolean;
  };
  export let settingsError: string | null = null;
  export let onWarningPrefChange: (kind: "red" | "broken", enabled: boolean) => void | Promise<void>;
  export let onResetAllData: () => void | Promise<void>;

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

    <div class="settings-section danger-zone">
      <div class="settings-subheading">
        <h3>Danger zone</h3>
      </div>

      <div class="switch-row danger-row">
        <div>
          <strong>Reset all data</strong>
          <p>Clear profiles, settings, local overrides, and cached catalog metadata.</p>
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
