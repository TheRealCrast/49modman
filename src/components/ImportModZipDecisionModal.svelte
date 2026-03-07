<script lang="ts">
  import type { ImportProfileModZipPreviewResult } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let preview: ImportProfileModZipPreviewResult;
  export let onCancel: () => void;
  export let onImportOnly: () => void;
  export let onImportAndCache: () => void;

  $: importedMod = preview.importedMod;
</script>

<div class="modal-scrim" role="presentation">
  <section aria-modal="true" class="modal-card import-mod-zip-modal" role="dialog">
    <div class="compact-heading">
      <Icon label="Import mod zip" name="upload" />
      <h2>Import selected mod archive?</h2>
    </div>

    {#if importedMod}
      <p class="modal-copy">
        <strong>{importedMod.packageName}</strong> {importedMod.versionNumber} was identified in the selected archive.
      </p>
    {:else}
      <p class="modal-copy">The selected .zip archive is ready to import into the active profile.</p>
    {/if}

    {#if preview.sourcePath}
      <div class="modal-note">
        <p class="dependants-title">Selected archive</p>
        <p class="modal-path">{preview.sourcePath}</p>
      </div>
    {/if}

    <p class="warning-copy">
      Choose whether to also add this .zip archive to the shared cache while importing.
    </p>

    <div class="modal-actions">
      <button class="ghost-button icon-button" type="button" on:click={onCancel}>
        <Icon label="Cancel import" name="x-close" />
        <span>Cancel import</span>
      </button>
      <button class="ghost-button icon-button" type="button" on:click={onImportOnly}>
        <Icon label="Import only" name="upload" />
        <span>Import only</span>
      </button>
      <button class="solid-button icon-button" type="button" on:click={onImportAndCache}>
        <Icon label="Import and cache" name="download" forceWhite={true} />
        <span>Import and cache</span>
      </button>
    </div>
  </section>
</div>
