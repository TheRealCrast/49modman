<script lang="ts">
  import type { PreviewExportProfilePackResult } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let preview: PreviewExportProfilePackResult;
  export let isExporting = false;
  export let onCancel: () => void;
  export let onConfirm: (embedUnavailablePayloads: boolean) => void;

  function formatReason(reason: string) {
    if (reason === "missingVersion") {
      return "Missing from local catalog";
    }
    if (reason === "missingDownloadUrl") {
      return "Missing download URL";
    }
    return "Unavailable";
  }

  $: unavailableMods = preview.unavailableMods ?? [];
  $: unavailableCount = unavailableMods.length;
</script>

<div class="modal-scrim" role="presentation">
  <section aria-modal="true" class="modal-card export-pack-modal" role="dialog">
    <div class="compact-heading">
      <Icon label="Warning" name="warning" />
      <h2>Unavailable versions found</h2>
    </div>

    <p class="modal-copy">
      <strong>{preview.profileName}</strong> includes {unavailableCount} {unavailableCount === 1 ? "mod version" : "mod versions"} that may no longer be downloadable.
    </p>

    <p class="warning-copy">
      Choose whether to include fallback payloads for these entries before export.
    </p>

    <div class="modal-note">
      <p class="dependants-title">Unavailable versions</p>
      <ul class="export-pack-list">
        {#each unavailableMods as mod (`${mod.packageId}:${mod.versionId}`)}
          <li>
            <strong>{mod.packageName}</strong>
            <span>{mod.versionNumber}</span>
            <span class="dependant-depth">{formatReason(mod.unavailableReason)}</span>
          </li>
        {/each}
      </ul>
    </div>

    <div class="modal-actions">
      <button class="ghost-button icon-button" type="button" disabled={isExporting} on:click={onCancel}>
        <Icon label="Cancel" name="x-close" />
        <span>Cancel</span>
      </button>
      <button
        class="ghost-button icon-button"
        type="button"
        disabled={isExporting}
        on:click={() => onConfirm(false)}
      >
        <Icon label="No" name="circle" />
        <span>No</span>
      </button>
      <button
        class="solid-button icon-button"
        type="button"
        disabled={isExporting}
        on:click={() => onConfirm(true)}
      >
        <Icon label="Yes" name={isExporting ? "refresh" : "check"} spinning={isExporting} forceWhite={true} />
        <span>{isExporting ? "Exporting..." : "Yes"}</span>
      </button>
    </div>
  </section>
</div>
