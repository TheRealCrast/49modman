<script lang="ts">
  import type { DownloadJobDto } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let downloads: DownloadJobDto[] = [];

  function formatBytes(value?: number) {
    if (value === undefined) {
      return "";
    }

    if (value < 1024) {
      return `${value} B`;
    }

    if (value < 1024 * 1024) {
      return `${(value / 1024).toFixed(0)} KB`;
    }

    return `${(value / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatTransfer(download: DownloadJobDto) {
    if (download.cacheHit) {
      return "copied from cache";
    }

    if (download.totalBytes !== undefined && download.totalBytes > 0) {
      return `${download.bytesDownloaded}/${download.totalBytes} bytes`;
    }

    return formatBytes(download.bytesDownloaded);
  }
</script>

<section class="screen-stack download-screen">
  <section class="panel list-panel">
    <div class="compact-heading compact-heading-left">
      <Icon label="Downloads" name="download" />
      <h2>Downloads</h2>
    </div>

    <p class="download-summary">
      All downloaded mod archives live in one shared cache. If a profile needs a version that is already cached, it can be copied in locally instead of downloading it again.
    </p>

    <div class="download-list list-scroll">
      {#if downloads.length === 0}
        <div class="list-state panel compact-list-state">
          <p>No active downloads.</p>
        </div>
      {:else}
        {#each downloads as download}
          <article class="download-card">
            <div class="download-primary">
              <strong>{download.packageName}</strong>
              <p>Version {download.versionLabel}</p>
            </div>
            <div class="download-secondary">
              <span class={`download-status ${download.status}`}>{download.status}</span>
              <p>{download.progressLabel}</p>
            </div>
            <div class="download-tertiary">
              <span>{formatTransfer(download)}</span>
              <p>{download.cacheHit ? "cache hit" : download.speedBps ? `${formatBytes(download.speedBps)}/s` : "thunderstore"}</p>
            </div>
          </article>
        {/each}
      {/if}
    </div>
  </section>
</section>
