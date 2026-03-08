<script lang="ts">
  import type { StorageMigrationStatusDto } from "../lib/types";

  export let status: StorageMigrationStatusDto;

  function clampPercent(value: number) {
    if (!Number.isFinite(value)) {
      return 0;
    }
    return Math.max(0, Math.min(100, value));
  }

  function formatBytes(value = 0) {
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

  $: percent = clampPercent(status.percentComplete);
  $: hasTotal = status.totalBytes > 0;
</script>

<div class="modal-scrim storage-migration-scrim" role="presentation">
  <section aria-modal="true" class="modal-card storage-migration-card" role="dialog">
    <div class="loading-spinner" aria-hidden="true"></div>
    <h2>Moving storage data</h2>
    <p>{status.message}</p>

    <div class="storage-progress">
      <div class="storage-progress-bar" aria-label="Storage copy progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent}>
        <span style={`width: ${percent.toFixed(1)}%`}></span>
      </div>
      <div class="storage-progress-meta">
        <strong>{percent.toFixed(1)}%</strong>
        <span>{hasTotal ? `${formatBytes(status.bytesCopied)} / ${formatBytes(status.totalBytes)}` : "Preparing files..."}</span>
      </div>
    </div>
  </section>
</div>
