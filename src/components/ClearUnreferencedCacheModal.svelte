<script lang="ts">
  import type { CachePrunePreviewDto } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let preview: CachePrunePreviewDto;
  export let onCancel: () => void;
  export let onConfirm: () => void;

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
</script>

<div class="modal-scrim" role="presentation">
  <section aria-modal="true" class="modal-card clear-cache-modal" role="dialog">
    <div class="compact-heading">
      <Icon label="Warning" name="warning" />
      <h2>Clear unreferenced cache?</h2>
    </div>

    <p class="modal-copy">
      {preview.removableCount} cached {preview.removableCount === 1 ? "archive" : "archives"} will be
      removed ({formatDiskSpace(preview.removableBytes)}). Installed versions are kept even when disabled.
    </p>

    <div class="modal-note">
      <p class="dependants-title">Mod versions to remove</p>
      <ul class="cache-prune-list">
        {#each preview.candidates as candidate (`${candidate.versionId}:${candidate.archiveName}`)}
          <li class="cache-prune-item">
            <div class="cache-prune-main">
              <strong>{candidate.packageName}</strong>
              <span class="cache-prune-version">{candidate.versionNumber}</span>
            </div>
            <span class="cache-prune-size">{formatDiskSpace(candidate.fileSize)}</span>
          </li>
        {/each}
      </ul>
    </div>

    <div class="modal-actions">
      <button class="ghost-button icon-button" type="button" on:click={onCancel}>
        <Icon label="Cancel" name="x-close" />
        <span>Cancel</span>
      </button>
      <button class="danger-button icon-button" type="button" on:click={onConfirm}>
        <Icon label="Clear cache" name="trash" forceWhite={true} />
        <span>Clear unreferenced</span>
      </button>
    </div>
  </section>
</div>
