<script lang="ts">
  import { resolveEffectiveStatus } from "../lib/status";
  import type { ModPackage } from "../lib/types";
  import Icon from "./Icon.svelte";
  import StatusPill from "./StatusPill.svelte";

  export let pkg: ModPackage | undefined;
  export let visibleStatuses: string[];
  export let onToggleStatus: (status: "verified" | "broken" | "green" | "yellow" | "orange" | "red") => void;
  export let onInstall: (packageId: string, versionId: string) => void;
  export let onSetReference: (packageId: string, versionId: string, state: "verified" | "broken" | "neutral") => void;

  const filters = ["verified", "green", "yellow", "orange", "red", "broken"] as const;
</script>

{#if pkg}
  <section class="panel detail-panel list-panel detail-panel-grid">
    <div class="detail-header detail-header-left">
      <div>
        <h3>{pkg.fullName}</h3>
        <p>{pkg.summary}</p>
      </div>

      <div class="detail-metrics">
        <span>{pkg.versions.length} versions</span>
      </div>
    </div>

    <div class="chip-row">
      {#each pkg.categories as category}
        <span class="category-chip">{category}</span>
      {/each}
    </div>

    <div class="filter-row">
      {#each filters as filter}
        <button
          class:active={visibleStatuses.includes(filter)}
          class={`toggle-chip ${filter}`}
          type="button"
          on:click={() => onToggleStatus(filter)}
        >
          {filter}
        </button>
      {/each}
    </div>

    <div class="versions-list list-scroll">
      {#each [...pkg.versions].sort((left, right) => right.publishedAt.localeCompare(left.publishedAt)) as version}
        {#if visibleStatuses.includes(version.effectiveStatus ?? resolveEffectiveStatus(version))}
          <article class="version-row">
            <div class="version-main">
              <div class="version-title">
                <strong>{version.versionNumber}</strong>
                <StatusPill status={version.effectiveStatus ?? resolveEffectiveStatus(version)} />
              </div>
              <p>
                Published {version.publishedAt} · {version.downloads.toLocaleString()} downloads
              </p>

              {#if version.overrideReferenceState === "verified" || version.overrideReferenceState === "broken"}
                <p class="reference-note">
                  Local override: {version.overrideReferenceNote}
                </p>
              {:else if version.bundledReferenceState}
                <p class="reference-note">
                  Bundled reference: {version.bundledReferenceNote}
                </p>
              {/if}
            </div>

            <div class="version-actions">
              <button class="ghost-button icon-button" type="button" on:click={() => onSetReference(pkg.id, version.id, "verified")}>
                <Icon label="Mark verified" name="verified" />
                <span>Verified</span>
              </button>
              <button class="ghost-button danger-outline icon-button" type="button" on:click={() => onSetReference(pkg.id, version.id, "broken")}>
                <Icon label="Mark broken" name="broken" />
                <span>Broken</span>
              </button>
              <button class="ghost-button icon-button" type="button" on:click={() => onSetReference(pkg.id, version.id, "neutral")}>
                <Icon label="Clear override" name="x-close" />
                <span>Clear</span>
              </button>
              <button class="solid-button icon-button" type="button" on:click={() => onInstall(pkg.id, version.id)}>
                <Icon label="Install version" name="download" />
                <span>Install</span>
              </button>
            </div>
          </article>
        {/if}
      {/each}
    </div>
  </section>
{:else}
  <section class="panel detail-panel empty-panel">
    <h3>Select a mod</h3>
    <p>Choose a package from the browser to inspect its versions, local references, and install warnings.</p>
  </section>
{/if}
