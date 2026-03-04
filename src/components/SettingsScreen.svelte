<script lang="ts">
  import Icon from "./Icon.svelte";
  export let warningPrefs: {
    red: boolean;
    broken: boolean;
  };
  export let onWarningPrefChange: (kind: "red" | "broken", enabled: boolean) => void;
</script>

<section class="screen-stack">
  <section class="panel compact-panel">
    <div class="compact-heading compact-heading-left">
      <Icon label="Settings" name="settings" />
      <h2>Global settings</h2>
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
  </section>
</section>
