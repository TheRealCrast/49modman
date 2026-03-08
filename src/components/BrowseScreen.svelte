<script lang="ts">
  import type {
    BrowseSortMode,
    EffectiveStatus,
    InstallActionOptions,
    InstallRequest,
    ModPackage,
    ProfileInstalledModDto
  } from "../lib/types";
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
    recommendedVersionId: string;
    recommendedVersion: string;
    iconUrl?: string;
    effectiveStatus: EffectiveStatus;
    everyRelevantVersionBroken: boolean;
  }> = [];
  export let selectedPackage: ModPackage | undefined;
  export let searchDraft = "";
  export let browseSortMode: BrowseSortMode = "mostDownloads";
  export let visibleStatuses: EffectiveStatus[] = [];
  export let busyPackageIds: string[] = [];
  export let isRefreshingCatalog = false;
  export let isResourceSaverActive = false;
  export let isLoadingFirstPage = false;
  export let isLoadingNextPage = false;
  export let hasMore = false;
  export let catalogError: string | null = null;
  export let refreshLabel = "";
  export let onSearchDraftChange: (value: string) => void;
  export let onSubmitSearch: () => void;
  export let onBrowseSortChange: (sortMode: BrowseSortMode) => void;
  export let onRefresh: () => void;
  export let onLoadMore: () => void;
  export let onToggleStatus: (status: EffectiveStatus) => void;
  export let onSelectPackage: (packageId: string) => void;
  export let onInstall: (request: InstallRequest, options?: InstallActionOptions) => void;
  export let onSwitchVersion: (
    request: InstallRequest,
    switchFromVersionIds: string[],
    options?: InstallActionOptions
  ) => void;
  export let onUninstallPackage: (packageId: string, packageName: string) => void;
  export let onUninstallVersion: (
    packageId: string,
    versionId: string,
    packageName: string,
    versionNumber: string
  ) => void;
  export let installedMods: ProfileInstalledModDto[] = [];
  export let onSetReference: (packageId: string, versionId: string, state: "verified" | "broken" | "neutral") => void;
  export let onViewDependencies: (request: {
    packageId: string;
    packageName: string;
    versionId: string;
    versionNumber: string;
  }) => void;
  export let focusedVersionId: string | undefined = undefined;
  export let focusedVersionToken = 0;

  const filters: EffectiveStatus[] = ["verified", "green", "yellow", "orange", "red", "broken"];
  let listElement: HTMLElement | undefined;
  let autoloadQueued = false;
  let selectedPackageBusy = false;
  let detailLockMessage = "Searching cached mods...";
  let isListInteractionLocked = false;

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
    if (isResourceSaverActive) {
      return;
    }

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

  $: if (
    listElement &&
    hasMore &&
    !isLoadingFirstPage &&
    !isLoadingNextPage &&
    !isResourceSaverActive &&
    !autoloadQueued
  ) {
    if (listElement.scrollHeight <= listElement.clientHeight + 40) {
      autoloadQueued = true;
      requestAnimationFrame(() => {
        autoloadQueued = false;
        onLoadMore();
      });
    }
  }

  function isPackageInstalled(packageId: string) {
    return installedMods.some((entry) => entry.packageId === packageId);
  }

  function isPackageBusy(packageId: string) {
    return busyPackageIds.includes(packageId);
  }

  function handleCardPrimaryAction(card: (typeof cards)[number]) {
    if (isResourceSaverActive) {
      return;
    }

    if (isPackageBusy(card.id)) {
      return;
    }

    if (isPackageInstalled(card.id)) {
      onUninstallPackage(card.id, card.fullName);
      return;
    }

    onInstall(buildCardInstallRequest(card));
  }

  $: selectedPackageBusy = selectedPackage ? isPackageBusy(selectedPackage.id) : false;
  $: detailLockMessage = selectedPackageBusy ? "Waiting..." : "Searching cached mods...";
  $: isListInteractionLocked = isLoadingFirstPage && cards.length > 0;
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
            disabled={isResourceSaverActive}
            on:input={(event) => onSearchDraftChange((event.currentTarget as HTMLInputElement).value)}
          />
        </label>

        <label class="settings-select browse-sort-select">
          <span>Sort</span>
          <select
            disabled={isResourceSaverActive}
            value={browseSortMode}
            on:change={(event) =>
              onBrowseSortChange((event.currentTarget as HTMLSelectElement).value as BrowseSortMode)}
          >
            <option value="mostDownloads">Most downloads</option>
            <option value="compatibility">Compatibility</option>
            <option value="lastUpdated">Last updated</option>
            <option value="nameAsc">A-Z</option>
            <option value="nameDesc">Z-A</option>
          </select>
        </label>

        <button class="ghost-button icon-button" type="submit" disabled={isResourceSaverActive}>
          <Icon label="Search" name="search" />
          <span>Search</span>
        </button>

        <button
          class="ghost-button icon-button"
          disabled={isRefreshingCatalog || isResourceSaverActive}
          type="button"
          on:click={onRefresh}
        >
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
            disabled={isResourceSaverActive}
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

    <div class="card-list-wrap">
      <section
        class="card-list list-scroll"
        class:list-interaction-locked={isListInteractionLocked}
        aria-busy={isListInteractionLocked}
        bind:this={listElement}
        on:scroll={handleListScroll}
      >
        {#if isLoadingFirstPage && cards.length === 0}
          <div class="list-state panel browse-status-padded">
            <p>Loading mods...</p>
          </div>
        {:else if catalogError && cards.length === 0}
          <div class="list-state panel">
            <p>{catalogError}</p>
            <button class="ghost-button icon-button" type="button" on:click={onRefresh}>
              <Icon label="Refresh" name="refresh" />
              <span>Retry</span>
            </button>
          </div>
        {:else if cards.length === 0 && isResourceSaverActive}
          <div class="list-state panel browse-status-padded">
            <p>Resource saver is active while the game is running.</p>
          </div>
        {:else if cards.length === 0}
          <div class="list-state panel browse-status-padded">
            <p>No mods matched this search.</p>
          </div>
        {:else}
          {#each cards as card}
            {@const packageInstalled = isPackageInstalled(card.id)}
            {@const packageBusy = isPackageBusy(card.id)}
            <article class="package-card panel">
              <button
                class="package-card-select"
                type="button"
                disabled={isResourceSaverActive}
                on:click={() => onSelectPackage(card.id)}
              >
                <div class="package-card-header">
                  <div class="package-card-header-main">
                    {#if card.iconUrl}
                      <img alt={`${card.fullName} icon`} class="package-card-icon" src={card.iconUrl} loading="lazy" />
                    {:else}
                      <div aria-hidden="true" class="package-card-icon package-card-icon-fallback">
                        <span>{card.fullName.slice(0, 1).toUpperCase()}</span>
                      </div>
                    {/if}
                    <div class="package-card-title-block">
                      <p class="package-name">{card.fullName}</p>
                      <p class="package-meta">
                        by {card.author} • {card.totalDownloads.toLocaleString()} downloads
                      </p>
                    </div>
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
                  class={`solid-button icon-button package-install-button package-card-install-button ${packageBusy ? "busy" : packageInstalled ? "uninstall" : card.effectiveStatus}`}
                  type="button"
                  disabled={packageBusy || isResourceSaverActive}
                  aria-label={
                    packageBusy
                      ? `Working on ${card.fullName}`
                      : isResourceSaverActive
                      ? `${card.fullName} is unavailable while resource saver is active`
                      : packageInstalled
                      ? `Uninstall ${card.fullName}`
                      : `Install ${card.fullName} ${card.recommendedVersion}`
                  }
                  title={
                    packageBusy
                      ? "Working..."
                      : isResourceSaverActive
                      ? "Unavailable while resource saver is active"
                      : packageInstalled
                      ? "Uninstall"
                      : `Install ${card.recommendedVersion}`
                  }
                  on:click={() => handleCardPrimaryAction(card)}
                >
                  {#if packageBusy}
                    <div class="loading-spinner" aria-hidden="true"></div>
                  {:else}
                    <Icon
                      label={
                        packageInstalled
                          ? `Uninstall ${card.fullName}`
                          : `Install ${card.recommendedVersion}`
                      }
                      name={packageInstalled ? "trash" : "download"}
                      forceWhite={true}
                    />
                  {/if}
                  {#if !packageBusy}
                    <span>{packageInstalled ? "Uninstall" : "Install"}</span>
                  {/if}
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

      {#if isListInteractionLocked}
        <div class="detail-lock-overlay browse-list-lock-overlay" aria-live="polite">
          <div class="detail-lock-card">
            <div class="loading-spinner" aria-hidden="true"></div>
            <p>Searching cached mods...</p>
          </div>
        </div>
      {/if}
    </div>
  </div>

  <PackageDetail
    focusedVersionId={focusedVersionId}
    focusedVersionToken={focusedVersionToken}
    isLocked={isLoadingFirstPage || selectedPackageBusy}
    lockMessage={detailLockMessage}
    pkg={selectedPackage}
    visibleStatuses={visibleStatuses}
    onToggleStatus={onToggleStatus}
    onInstall={onInstall}
    onSwitchVersion={onSwitchVersion}
    onUninstallPackage={onUninstallPackage}
    onUninstallVersion={onUninstallVersion}
    onSetReference={onSetReference}
    onViewDependencies={onViewDependencies}
    installedMods={installedMods}
  />
</section>
