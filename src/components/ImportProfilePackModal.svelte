<script lang="ts">
  import type { ImportProfilePackPreviewResult } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let preview: ImportProfilePackPreviewResult;
  export let isImporting = false;
  export let onCancel: () => void;
  export let onConfirm: (doNotShowAgain: boolean) => void;

  let doNotShowAgain = false;

  $: listedMods = preview.mods ?? [];
  $: modCount = listedMods.length;
  $: profileName = preview.profileName?.trim() || "Imported profile";
  $: payloadModeLabel =
    preview.payloadMode === "full"
      ? "Full payload pack"
      : preview.payloadMode === "hybrid"
        ? "Hybrid payload pack"
        : "Compact metadata pack";
</script>

<div class="modal-scrim" role="presentation">
  <section aria-modal="true" class="modal-card import-pack-modal" role="dialog">
    <div class="compact-heading">
      <Icon label="Import profile pack" name="folder" />
      <h2>Import .49pack?</h2>
    </div>

    <p class="modal-copy">
      <strong>{profileName}</strong> will import {modCount} {modCount === 1 ? "mod" : "mods"}.
    </p>
    <p class="warning-copy">
      {payloadModeLabel} ({preview.embeddedModCount}/{preview.referencedModCount} mod payloads embedded).
    </p>
    {#if preview.hasLegacyRuntimePluginsPayload}
      <p class="warning-copy">This pack also includes legacy runtime plugin payload files.</p>
    {/if}

    <div class="modal-note">
      <p class="dependants-title">Mods to install</p>
      {#if modCount === 0}
        <p class="warning-copy">No mods were listed in this pack.</p>
      {:else}
        <ul class="import-pack-list">
          {#each listedMods as mod (`${mod.packageId}:${mod.versionId}`)}
            <li>
              <strong>{mod.packageName}</strong>
              <span>{mod.versionNumber}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <button
      aria-pressed={doNotShowAgain}
      class="ghost-button icon-button toggle-icon-button modal-toggle"
      type="button"
      disabled={isImporting}
      on:click={() => (doNotShowAgain = !doNotShowAgain)}
    >
      <Icon label={doNotShowAgain ? "Enabled" : "Disabled"} name={doNotShowAgain ? "check" : "circle"} />
      <span>Do not show this again</span>
    </button>

    <div class="modal-actions">
      <button class="ghost-button icon-button" type="button" disabled={isImporting} on:click={onCancel}>
        <Icon label="Cancel" name="x-close" />
        <span>Cancel</span>
      </button>
      <button class="solid-button icon-button" type="button" disabled={isImporting} on:click={() => onConfirm(doNotShowAgain)}>
        <Icon label="Import profile" name={isImporting ? "refresh" : "download"} spinning={isImporting} />
        <span>{isImporting ? "Importing..." : "Import profile"}</span>
      </button>
    </div>
  </section>
</div>
