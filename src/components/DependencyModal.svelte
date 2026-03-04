<script lang="ts">
  import type { DependencyModalState } from "../lib/types";
  import Icon from "./Icon.svelte";
  import DependencyTreeNode from "./DependencyTreeNode.svelte";

  export let state: DependencyModalState;
  export let onClose: () => void;
  export let onJumpToDependency: (packageId: string, versionId: string) => void;

  function handleScrimClick(event: MouseEvent) {
    if (event.currentTarget === event.target) {
      onClose();
    }
  }
</script>

<div class="modal-scrim" role="presentation" on:click={handleScrimClick}>
  <section aria-modal="true" class="modal-card dependency-modal" role="dialog">
    <div class="dependency-modal-header">
      <div class="compact-heading compact-heading-left">
        <Icon label="Dependency details" name="details" />
        <div>
          <h2>Dependencies for {state.packageName} {state.versionNumber}</h2>
          <p class="dependency-modal-subtitle">Resolved from the local cached catalog.</p>
        </div>
      </div>

      <button class="ghost-button icon-button" type="button" on:click={onClose}>
        <Icon label="Close" name="x-close" />
        <span>Close</span>
      </button>
    </div>

    {#if state.isLoading}
      <div class="dependency-modal-state">
        <div class="loading-spinner" aria-hidden="true"></div>
        <p>Resolving dependency tree...</p>
      </div>
    {:else if state.error}
      <div class="dependency-modal-state dependency-modal-state-error">
        <p>{state.error}</p>
      </div>
    {:else if !state.tree || state.tree.items.length === 0}
      <div class="dependency-modal-state">
        <p>This version does not declare any dependencies in the cached catalog.</p>
      </div>
    {:else}
      <div class="dependency-tree">
        {#each state.tree.items as node}
          <DependencyTreeNode {node} depth={0} {onJumpToDependency} />
        {/each}
      </div>
    {/if}
  </section>
</div>
