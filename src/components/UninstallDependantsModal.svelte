<script lang="ts">
  import type { UninstallDependantDto } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let packageName: string;
  export let dependants: UninstallDependantDto[];
  export let onCancel: () => void;
  export let onConfirm: (doNotShowAgain: boolean) => void;

  let doNotShowAgain = false;

  $: dependantCount = dependants.length;
  $: dependantWord = dependantCount === 1 ? "mod that depends" : "mods that depend";
</script>

<div class="modal-scrim" role="presentation">
  <section aria-modal="true" class="modal-card" role="dialog">
    <div class="compact-heading">
      <Icon label="Warning" name="warning" />
      <h2>Uninstall dependency mod?</h2>
    </div>
    <p class="modal-copy">
      {packageName} is required by {dependantCount} installed {dependantWord} on it.
      Continuing may leave those mods broken in this profile.
    </p>

    <div class="modal-note">
      <p class="dependants-title">Installed dependants</p>
      <ul class="dependants-list">
        {#each dependants as dependant}
          <li>
            <strong>{dependant.packageName}</strong>
            <span>{dependant.versionNumber}</span>
            <span class="dependant-depth">{dependant.minDepth === 1 ? "Direct" : "Indirect"}</span>
          </li>
        {/each}
      </ul>
    </div>

    <button
      aria-pressed={doNotShowAgain}
      class="ghost-button icon-button toggle-icon-button modal-toggle"
      type="button"
      on:click={() => (doNotShowAgain = !doNotShowAgain)}
    >
      <Icon label={doNotShowAgain ? "Enabled" : "Disabled"} name={doNotShowAgain ? "check" : "circle"} />
      <span>Do not show this again</span>
    </button>

    <div class="modal-actions">
      <button class="ghost-button icon-button" type="button" on:click={onCancel}>
        <Icon label="Cancel" name="x-close" />
        <span>Cancel</span>
      </button>
      <button class="danger-button icon-button" type="button" on:click={() => onConfirm(doNotShowAgain)}>
        <Icon label="Uninstall anyway" name="trash" />
        <span>Uninstall anyway</span>
      </button>
    </div>
  </section>
</div>
