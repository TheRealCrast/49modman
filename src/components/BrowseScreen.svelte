<script lang="ts">
  import type { EffectiveStatus, InstallRequest, ModPackage } from "../lib/types";
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
  export let isLoadingFirstPage = false;
  export let isLoadingNextPage = false;
  export let hasMore = false;
  export let catalogError: string | null = null;
  export let refreshLabel = "";
  export let onSearchDraftChange: (value: string) => void;
  export let onSubmitSearch: () => void;
  export let onRefresh: () => void;
  export let onLoadMore: () => void;
  export let onToggleStatus: (status: EffectiveStatus) => void;
  export let onSelectPackage: (packageId: string) => void;
  export let onInstall: (request: InstallRequest) => void;
  export let onSetReference: (packageId: string, versionId: string, state: "verified" | "broken" | "neutral") => void;

  const filters: EffectiveStatus[] = ["verified", "green", "yellow", "orange", "red", "broken"];
  let listElement: HTMLElement | undefined;
  let autoloadQueued = false;

  function buildCardInstallRequest(card: (typeof cards)[number]): InstallRequest {
    return {
      packageId: card.id,
      packageName: card.fullName,
      versionId: card.recommendedVersionId,
      versionNumber: card.recommendedVersion,
      effectiveStatus: card.effectiveStatus
    };
  }

  function handleListScroll(event: Event) {
    const target = event.currentTarget as HTMLElement;

    if (
      hasMore &&
      !isLoadingFirstPage &&
      !isLoadingNextPage &&
      target.scrollTop + target.clientHeight >= target.scrollHeight - 240
    ) {
      onLoadMore();
    }
  }

  $: if (listElement && hasMore && !isLoadingFirstPage && !isLoadingNextPage && !autoloadQueued) {
    if (listElement.scrollHeight <= listElement.clientHeight + 40) {
      autoloadQueued = true;
      requestAnimationFrame(() => {
        autoloadQueued = false;
        onLoadMore();
      });
    }
  }
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

    <section class="card-list list-scroll" bind:this={listElement} on:scroll={handleListScroll}>
      {#if isLoadingFirstPage && cards.length === 0}
        <div class="list-state panel">
          <p>Loading mod list...</p>
        </div>
      {:else if catalogError && cards.length === 0}
        <div class="list-state panel">
          <p>{catalogError}</p>
          <button class="ghost-button icon-button" type="button" on:click={onRefresh}>
            <Icon label="Refresh" name="refresh" />
            <span>Retry</span>
          </button>
        </div>
      {:else if cards.length === 0}
        <div class="list-state panel">
          <p>No mods matched this search.</p>
        </div>
      {:else}
        {#if isLoadingFirstPage}
          <div class="list-state compact-list-state panel">
            <p>Searching cached mods...</p>
          </div>
        {/if}

        {#each cards as card}
          <article class="package-card panel">
            <button class="package-card-select" type="button" on:click={() => onSelectPackage(card.id)}>
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
            </button>

            <div class="package-card-footer">
              <div class="package-card-footer-copy">
                <span>Recommended {card.recommendedVersion}</span>
                {#if card.everyRelevantVersionBroken}
                  <span class="warning-copy danger">Broken candidates only</span>
                {/if}
              </div>

              <button
                class={`solid-button icon-button package-install-button package-card-install-button ${card.effectiveStatus}`}
                type="button"
                aria-label={`Install ${card.fullName} ${card.recommendedVersion}`}
                title={`Install ${card.recommendedVersion}`}
                on:click={() => onInstall(buildCardInstallRequest(card))}
              >
                <Icon label={`Install ${card.recommendedVersion}`} name="download" />
              </button>
            </div>
          </article>
        {/each}

        {#if catalogError}
          <div class="list-state compact-list-state panel">
            <p>{catalogError}</p>
            <button class="ghost-button icon-button" type="button" on:click={onRefresh}>
              <Icon label="Refresh" name="refresh" />
              <span>Retry</span>
            </button>
          </div>
        {/if}

        {#if isLoadingNextPage}
          <div class="list-state compact-list-state panel">
            <p>Loading more mods...</p>
          </div>
        {/if}

        {#if !hasMore && cards.length > 0}
          <div class="list-state compact-list-state panel">
            <p>End of cached results.</p>
          </div>
        {/if}

        <div class="list-sentinel"></div>
      {/if}
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
