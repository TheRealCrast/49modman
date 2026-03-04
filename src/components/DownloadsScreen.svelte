<script lang="ts">
  import type { DownloadItem } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let downloads: DownloadItem[] = [];
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
      {#each downloads as download}
        <article class="download-card">
          <div class="download-primary">
            <strong>{download.packageName}</strong>
            <p>Version {download.versionNumber}</p>
          </div>
          <div class="download-secondary">
            <span class={`download-status ${download.status}`}>{download.status}</span>
            <p>{download.progressLabel}</p>
          </div>
          <div class="download-tertiary">
            <span>{download.speedLabel}</span>
            <p>{download.cacheHit ? "cache hit" : "network path pending"}</p>
          </div>
        </article>
      {/each}
    </div>
  </section>
</section>
