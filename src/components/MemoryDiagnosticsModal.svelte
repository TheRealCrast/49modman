<script lang="ts">
  import type { MemoryDiagnosticsModalState } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let state: MemoryDiagnosticsModalState;
  export let onClose: () => void;
  export let onRefresh: () => void;

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

  function formatTimestamp(value?: string) {
    if (!value) {
      return "";
    }

    const date = new Date(value);
    if (Number.isNaN(date.getTime())) {
      return value;
    }

    return date.toLocaleString();
  }

  function roleLabel(value: string) {
    if (value === "appMain") {
      return "App";
    }
    if (value === "webview") {
      return "WebView";
    }
    if (value === "network") {
      return "Network";
    }

    return "Child";
  }

  $: rows = [...(state.data?.processes ?? [])].sort((left, right) => right.rssBytes - left.rssBytes);
</script>

<div class="modal-scrim" role="presentation">
  <section aria-modal="true" class="modal-card memory-diagnostics-modal" role="dialog">
    <div class="memory-diagnostics-header">
      <div class="compact-heading compact-heading-left">
        <Icon label="RAM diagnostics" name="details" />
        <h2>RAM usage</h2>
      </div>
      <button class="ghost-button icon-button" type="button" on:click={onClose}>
        <Icon label="Close" name="x-close" />
        <span>Close</span>
      </button>
    </div>

    <p class="memory-diagnostics-note">Updates every 2 seconds while this window is open.</p>

    {#if state.data}
      <div class="memory-totals panel">
        <p>
          <strong>Total RSS:</strong> {formatDiskSpace(state.data.totals.rssBytes)}
        </p>
        <p>
          <strong>PSS:</strong>
          {state.data.totals.pssBytes !== undefined ? formatDiskSpace(state.data.totals.pssBytes) : "Unavailable"}
        </p>
        <p>
          <strong>Private:</strong>
          {state.data.totals.privateBytes !== undefined
            ? formatDiskSpace(state.data.totals.privateBytes)
            : "Unavailable"}
        </p>
        <p>
          <strong>Shared:</strong>
          {state.data.totals.sharedBytes !== undefined
            ? formatDiskSpace(state.data.totals.sharedBytes)
            : "Unavailable"}
        </p>
        <p>
          <strong>Captured:</strong> {formatTimestamp(state.data.capturedAt)}
        </p>
      </div>

      {#if state.data.notes.length > 0}
        <div class="memory-notes panel">
          {#each state.data.notes as note}
            <p>{note}</p>
          {/each}
        </div>
      {/if}

      <div class="memory-process-list">
        {#if rows.length === 0}
          <div class="list-state panel compact-list-state">
            <p>No matching processes were returned.</p>
          </div>
        {:else}
          {#each rows as process (process.pid)}
            <div class="panel memory-process-row">
              <div class="memory-process-main">
                <strong>{process.name}</strong>
                <span class="memory-process-meta">PID {process.pid} • {roleLabel(process.role)}</span>
              </div>
              <div class="memory-process-metrics">
                <span>RSS {formatDiskSpace(process.rssBytes)}</span>
                <span>
                  PSS {process.pssBytes !== undefined ? formatDiskSpace(process.pssBytes) : "n/a"}
                </span>
                <span>
                  Private
                  {process.privateBytes !== undefined ? formatDiskSpace(process.privateBytes) : "n/a"}
                </span>
                <span>
                  Shared
                  {process.sharedBytes !== undefined ? formatDiskSpace(process.sharedBytes) : "n/a"}
                </span>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    {:else}
      <div class="dependency-modal-state">
        <div class="loading-spinner" aria-hidden="true"></div>
        <p>Collecting process memory snapshot...</p>
      </div>
    {/if}

    {#if state.error}
      <p class="warning-copy danger">{state.error}</p>
    {/if}

    <div class="modal-actions">
      <button class="ghost-button icon-button" type="button" disabled={state.isLoading} on:click={onRefresh}>
        <Icon label="Refresh diagnostics" name="refresh" spinning={state.isLoading} />
        <span>{state.isLoading ? "Refreshing..." : "Refresh"}</span>
      </button>
    </div>
  </section>
</div>
