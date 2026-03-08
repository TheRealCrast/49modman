<script lang="ts">
  import type {
    DependencyNodeDto,
    DependencyModalState,
    DependencySummaryItemDto
  } from "../lib/types";
  import { compareVersionNumbers } from "../lib/status";
  import Icon from "./Icon.svelte";
  import DependencyTreeNode from "./DependencyTreeNode.svelte";
  import StatusPill from "./StatusPill.svelte";

  export let state: DependencyModalState;
  export let onClose: () => void;
  export let onJumpToDependency: (packageId: string, versionId: string) => void;

  let activeView: "summary" | "tree" = "summary";
  let lastStateKey = "";
  let directCount = 0;
  let transitiveCount = 0;
  let unresolvedCount = 0;
  let totalCount = 0;
  let treeHighestVersionByPackage: Record<string, string> = {};

  $: stateKey = `${state.packageId}:${state.versionId}`;
  $: if (stateKey !== lastStateKey) {
    lastStateKey = stateKey;
    activeView = "summary";
  }

  $: hasDependencies = Boolean(
    state.data &&
      (state.data.summary.direct.length > 0 ||
        state.data.summary.transitive.length > 0 ||
        state.data.summary.unresolved.length > 0 ||
        state.data.treeItems.length > 0)
  );
  $: directCount = state.data?.summary.direct.length ?? 0;
  $: transitiveCount = state.data?.summary.transitive.length ?? 0;
  $: unresolvedCount = state.data?.summary.unresolved.length ?? 0;
  $: totalCount = directCount + transitiveCount + unresolvedCount;
  $: treeHighestVersionByPackage = state.data
    ? buildHighestVersionByPackage(state.data.treeItems)
    : {};

  function handleScrimClick(event: MouseEvent) {
    if (event.currentTarget === event.target) {
      onClose();
    }
  }

  function jumpToSummaryDependency(item: DependencySummaryItemDto) {
    onJumpToDependency(item.packageId, item.versionId);
  }

  function summaryHelperText(item: DependencySummaryItemDto) {
    return item.minDepth <= 1 ? "Required directly" : "Required indirectly";
  }

  function dependencyWord(count: number) {
    return count === 1 ? "dependency" : "dependencies";
  }

  function dependencyWordUpper(count: number) {
    return dependencyWord(count).toUpperCase();
  }

  function collapsedVersionsText(item: DependencySummaryItemDto) {
    if (!item.collapsedVersionNumbers || item.collapsedVersionNumbers.length === 0) {
      return "";
    }

    return item.collapsedVersionNumbers.join(", ");
  }

  function buildHighestVersionByPackage(nodes: DependencyNodeDto[]) {
    const highestVersionByPackage: Record<string, string> = {};
    const stack = [...nodes];

    while (stack.length > 0) {
      const node = stack.pop();
      if (!node) {
        continue;
      }

      if (node.packageId && node.versionNumber) {
        const current = highestVersionByPackage[node.packageId];
        if (!current || compareVersionNumbers(node.versionNumber, current) > 0) {
          highestVersionByPackage[node.packageId] = node.versionNumber;
        }
      }

      if (node.children.length > 0) {
        stack.push(...node.children);
      }
    }

    return highestVersionByPackage;
  }
</script>

<div class="modal-scrim" role="presentation" on:click={handleScrimClick}>
  <section aria-modal="true" class="modal-card dependency-modal" role="dialog">
    <div class="dependency-modal-header">
      <div class="compact-heading compact-heading-left">
        <Icon label="Dependency details" name="details" />
        <div>
          <h2>Dependencies for {state.packageName} {state.versionNumber}</h2>
          {#if !state.isLoading && state.data}
            <p class="dependency-modal-subtitle">{totalCount} {dependencyWord(totalCount)} found.</p>
          {/if}
        </div>
      </div>

      <button class="ghost-button icon-button" type="button" on:click={onClose}>
        <Icon label="Close" name="x-close" />
        <span>Close</span>
      </button>
    </div>

    {#if !state.isLoading && !state.error && state.data}
      <div class="dependency-modal-switcher" role="tablist" aria-label="Dependency view">
        <button
          aria-selected={activeView === "summary"}
          class:active={activeView === "summary"}
          class="dependency-view-toggle"
          role="tab"
          type="button"
          on:click={() => (activeView = "summary")}
        >
          Summary
        </button>
        <button
          aria-selected={activeView === "tree"}
          class:active={activeView === "tree"}
          class="dependency-view-toggle"
          role="tab"
          type="button"
          on:click={() => (activeView = "tree")}
        >
          Tree
        </button>
      </div>

    {/if}

    {#if state.isLoading}
      <div class="dependency-modal-state">
        <div class="loading-spinner" aria-hidden="true"></div>
        <p>Resolving dependencies from the cached catalog...</p>
      </div>
    {:else if state.error}
      <div class="dependency-modal-state dependency-modal-state-error">
        <p>{state.error}</p>
      </div>
    {:else if !state.data || !hasDependencies}
      <div class="dependency-modal-state">
        <p>This version does not declare any dependencies in the cached catalog.</p>
      </div>
    {:else if activeView === "summary"}
      <div class="dependency-summary">
        {#if state.data.summary.direct.length > 0}
          <section class="dependency-summary-section">
            <h3>{directCount} DIRECT {dependencyWordUpper(directCount)}</h3>
            <div class="dependency-summary-list">
              {#each state.data.summary.direct as item}
                <button
                  class="dependency-row dependency-row-button dependency-summary-row"
                  type="button"
                  on:click={() => jumpToSummaryDependency(item)}
                >
                  <div class="dependency-main">
                    <span class="dependency-line">
                      <strong>{item.packageName}</strong>
                      <span class="dependency-version">{item.versionNumber}</span>
                    </span>
                    <span class="dependency-note">{summaryHelperText(item)}</span>
                    {#if item.collapsedVersionNumbers.length > 0}
                      <span class="dependency-note dependency-collapsed-versions">
                        <s>{collapsedVersionsText(item)}</s>
                      </span>
                    {/if}
                    {#if item.referenceNote}
                      <span class="dependency-note">{item.referenceNote}</span>
                    {/if}
                  </div>
                  <div class="dependency-side">
                    <StatusPill compact={true} status={item.effectiveStatus} />
                  </div>
                </button>
              {/each}
            </div>
          </section>
        {/if}

        {#if state.data.summary.transitive.length > 0}
          <section class="dependency-summary-section">
            <h3>{transitiveCount} INDIRECT {dependencyWordUpper(transitiveCount)}</h3>
            <div class="dependency-summary-list">
              {#each state.data.summary.transitive as item}
                <button
                  class="dependency-row dependency-row-button dependency-summary-row"
                  type="button"
                  on:click={() => jumpToSummaryDependency(item)}
                >
                  <div class="dependency-main">
                    <span class="dependency-line">
                      <strong>{item.packageName}</strong>
                      <span class="dependency-version">{item.versionNumber}</span>
                    </span>
                    <span class="dependency-note">{summaryHelperText(item)}</span>
                    {#if item.collapsedVersionNumbers.length > 0}
                      <span class="dependency-note dependency-collapsed-versions">
                        <s>{collapsedVersionsText(item)}</s>
                      </span>
                    {/if}
                    {#if item.referenceNote}
                      <span class="dependency-note">{item.referenceNote}</span>
                    {/if}
                  </div>
                  <div class="dependency-side">
                    <StatusPill compact={true} status={item.effectiveStatus} />
                  </div>
                </button>
              {/each}
            </div>
          </section>
        {/if}

        {#if state.data.summary.unresolved.length > 0}
          <section class="dependency-summary-section">
            <h3>{unresolvedCount} UNRESOLVED {dependencyWordUpper(unresolvedCount)}</h3>
            <div class="dependency-summary-list">
              {#each state.data.summary.unresolved as item}
                <div class="dependency-row dependency-summary-row">
                  <div class="dependency-main">
                    <span class="dependency-line">
                      <strong>Unresolved dependency</strong>
                    </span>
                    <span class="dependency-meta">{item.raw}</span>
                    <span class="dependency-note">
                      {item.minDepth <= 1 ? "Required directly" : "Required indirectly"}
                    </span>
                  </div>
                  <div class="dependency-side">
                    <span class="dependency-state-pill unresolved">Unresolved</span>
                  </div>
                </div>
              {/each}
            </div>
          </section>
        {/if}
      </div>
    {:else}
      <div class="dependency-tree">
        {#each state.data.treeItems as node}
          <DependencyTreeNode
            {node}
            depth={0}
            {onJumpToDependency}
            highestVersionByPackage={treeHighestVersionByPackage}
          />
        {/each}
      </div>
    {/if}
  </section>
</div>
