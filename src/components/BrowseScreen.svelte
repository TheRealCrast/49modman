<script lang="ts">
  import type { EffectiveStatus, ModPackage } from "../lib/types";
  import Icon from "./Icon.svelte";
  import PackageDetail from "./PackageDetail.svelte";
  import StatusPill from "./StatusPill.svelte";

  export let cards: Array<{
    id: string;
    fullName: string;
    author: string;
    summary: string;
    categories: string[];
    totalDownloads: number;
    rating: number;
    versionCount: number;
    recommendedVersion: string;
    effectiveStatus: EffectiveStatus;
    everyRelevantVersionBroken: boolean;
  }> = [];
  export let selectedPackage: ModPackage | undefined;
  export let searchDraft = "";
  export let visibleStatuses: EffectiveStatus[] = [];
  export let isRefreshingCatalog = false;
  export let refreshLabel = "";
  export let onSearchDraftChange: (value: string) => void;
  export let onSubmitSearch: () => void;
  export let onRefresh: () => void;
  export let onToggleStatus: (status: EffectiveStatus) => void;
  export let onSelectPackage: (packageId: string) => void;
  export let onInstall: (packageId: string, versionId: string) => void;
  export let onSetReference: (packageId: string, versionId: string, state: "verified" | "broken" | "neutral") => void;

  const filters: EffectiveStatus[] = ["verified", "green", "yellow", "orange", "red", "broken"];
</script>

<section class="browse-grid">
  <div class="browser-column browse-main-column">
    <section class="panel browser-controls compact-panel">
      <div class="toolbar-heading toolbar-heading-left">
        <h2>Browse mods</h2>
      </div>
      <p class="toolbar-note toolbar-note-inline">{refreshLabel}</p>

      <form class="toolbar-row" on:submit|preventDefault={onSubmitSearch}>
        <label class="search-field search-inline">
          <Icon label="Search" name="search" />
          <input
            placeholder="Search by mod, author, category, or note"
            type="search"
            value={searchDraft}
            on:input={(event) => onSearchDraftChange((event.currentTarget as HTMLInputElement).value)}
          />
        </label>

        <button class="ghost-button icon-button" type="submit">
          <Icon label="Search" name="search" />
          <span>Search</span>
        </button>

        <button class="ghost-button icon-button" disabled={isRefreshingCatalog} type="button" on:click={onRefresh}>
          <Icon label="Refresh" name="refresh" />
          <span>{isRefreshingCatalog ? "Refreshing" : "Refresh"}</span>
        </button>
      </form>

      <div class="filter-row">
        {#each filters as filter}
          <button
            class:active={visibleStatuses.includes(filter)}
            class={`toggle-chip ${filter}`}
            type="button"
            on:click={() => onToggleStatus(filter)}
          >
            {#if filter === "verified"}
              <Icon label="Verified" name="verified" size={14} />
            {:else if filter === "broken"}
              <Icon label="Broken" name="broken" size={14} />
            {:else if filter === "red"}
              <Icon label="Warning" name="warning" size={14} />
            {:else}
              <Icon label="Filter" name="filter" size={14} />
            {/if}
            {filter}
          </button>
        {/each}
      </div>
    </section>

    <section class="card-list list-scroll">
      {#each cards as card}
        <button class="package-card panel" type="button" on:click={() => onSelectPackage(card.id)}>
          <div class="package-card-header">
            <div>
              <p class="package-name">{card.fullName}</p>
              <p class="package-meta">by {card.author}</p>
            </div>
            <StatusPill status={card.effectiveStatus} />
          </div>

          <p class="package-summary">{card.summary}</p>

          <div class="chip-row">
            {#each card.categories.slice(0, 3) as category}
              <span class="category-chip">{category}</span>
            {/each}
          </div>

          <div class="package-card-footer">
            <span>Recommended {card.recommendedVersion}</span>
            {#if card.everyRelevantVersionBroken}
              <span class="warning-copy danger">Broken candidates only</span>
            {/if}
          </div>
        </button>
      {/each}
    </section>
  </div>

  <PackageDetail
    pkg={selectedPackage}
    visibleStatuses={visibleStatuses}
    onToggleStatus={onToggleStatus}
    onInstall={onInstall}
    onSetReference={onSetReference}
  />
</section>
