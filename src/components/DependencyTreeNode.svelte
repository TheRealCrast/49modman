<script lang="ts">
  import type { DependencyNodeDto } from "../lib/types";
  import DependencyTreeNode from "./DependencyTreeNode.svelte";
  import StatusPill from "./StatusPill.svelte";

  export let node: DependencyNodeDto;
  export let depth = 0;
  export let onJumpToDependency: (packageId: string, versionId: string) => void;

  let hasChildren = false;
  let isResolved = false;
  let isExpanded = depth === 0;

  $: hasChildren = node.children.length > 0;
  $: isResolved = node.resolution === "resolved" && Boolean(node.packageId && node.versionId);

  function jumpToNode() {
    if (!node.packageId || !node.versionId) {
      return;
    }

    onJumpToDependency(node.packageId, node.versionId);
  }
</script>

<div class="dependency-node" style={`--dependency-depth:${depth};`}>
  <div class={`dependency-row ${isResolved ? "clickable" : ""}`}>
    {#if hasChildren}
      <button
        aria-label={isExpanded ? "Collapse dependencies" : "Expand dependencies"}
        aria-pressed={isExpanded}
        class="dependency-expander"
        type="button"
        on:click={() => (isExpanded = !isExpanded)}
      >
        {isExpanded ? "-" : "+"}
      </button>
    {:else}
      <span class="dependency-expander dependency-expander-placeholder" aria-hidden="true"></span>
    {/if}

    {#if isResolved && node.packageId && node.versionId}
      <button
        class="dependency-main dependency-main-button"
        type="button"
        on:click={jumpToNode}
      >
        <span class="dependency-line">
          <strong>{node.packageName}</strong>
          <span class="dependency-version">{node.versionNumber}</span>
        </span>
        <span class="dependency-meta">{node.raw}</span>
        {#if node.referenceNote}
          <span class="dependency-note">{node.referenceNote}</span>
        {/if}
      </button>
      <div class="dependency-side">
        {#if node.effectiveStatus}
          <StatusPill compact={true} status={node.effectiveStatus} />
        {/if}
      </div>
    {:else}
      <div class="dependency-main">
        <span class="dependency-line">
          <strong>{node.packageName ?? "Unresolved dependency"}</strong>
          {#if node.versionNumber}
            <span class="dependency-version">{node.versionNumber}</span>
          {/if}
        </span>
        <span class="dependency-meta">{node.raw}</span>
      </div>
      <div class="dependency-side">
        <span class={`dependency-state-pill ${node.resolution}`}>
          {node.resolution === "cycle" ? "Cycle" : "Unresolved"}
        </span>
      </div>
    {/if}
  </div>

  {#if hasChildren && isExpanded}
    <div class="dependency-children">
      {#each node.children as child}
        <DependencyTreeNode node={child} depth={depth + 1} {onJumpToDependency} />
      {/each}
    </div>
  {/if}
</div>
