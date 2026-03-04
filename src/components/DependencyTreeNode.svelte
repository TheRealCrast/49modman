<script lang="ts">
  import type { DependencyNodeDto } from "../lib/types";
  import { compareVersionNumbers } from "../lib/status";
  import DependencyTreeNode from "./DependencyTreeNode.svelte";
  import StatusPill from "./StatusPill.svelte";

  export let node: DependencyNodeDto;
  export let depth = 0;
  export let onJumpToDependency: (packageId: string, versionId: string) => void;
  export let highestVersionByPackage: Record<string, string> = {};

  let hasChildren = false;
  let isNavigable = false;
  let isExpanded = depth === 0;
  let stateLabel = "";
  let stateClass = "";
  let helperText = "";
  let isLowerVersion = false;

  $: hasChildren = node.resolution === "resolved" && node.children.length > 0;
  $: isNavigable =
    (node.resolution === "resolved" || node.resolution === "repeated") &&
    Boolean(node.packageId && node.versionId);
  $: stateLabel =
    node.resolution === "cycle"
      ? "Cycle"
      : node.resolution === "repeated"
        ? "Shown above"
        : "Unresolved";
  $: stateClass =
    node.resolution === "cycle"
      ? "cycle"
      : node.resolution === "repeated"
        ? "repeated"
        : "unresolved";
  $: helperText =
    node.resolution === "repeated" ? "Already shown above." : node.referenceNote ?? "";
  $: isLowerVersion = Boolean(
    node.packageId &&
      node.versionNumber &&
      highestVersionByPackage[node.packageId] &&
      compareVersionNumbers(node.versionNumber, highestVersionByPackage[node.packageId]) < 0
  );

  function jumpToNode() {
    if (!node.packageId || !node.versionId) {
      return;
    }

    onJumpToDependency(node.packageId, node.versionId);
  }
</script>

<div class="dependency-node" style={`--dependency-depth:${depth};`}>
  <div class={`dependency-row ${isNavigable ? "clickable" : ""} ${isLowerVersion ? "dependency-row-lower" : ""}`}>
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

    {#if isNavigable}
      <button
        class="dependency-main dependency-main-button"
        type="button"
        on:click={jumpToNode}
      >
        <span class="dependency-line">
          <strong class={isLowerVersion ? "dependency-title-lower" : ""}>{node.packageName}</strong>
          <span class={`dependency-version ${isLowerVersion ? "dependency-version-lower" : ""}`}>
            {node.versionNumber}
          </span>
        </span>
        <span class="dependency-meta">{node.raw}</span>
        {#if helperText}
          <span class="dependency-note">{helperText}</span>
        {/if}
      </button>
    {:else}
      <div class="dependency-main">
        <span class="dependency-line">
          <strong class={isLowerVersion ? "dependency-title-lower" : ""}>{node.packageName ?? "Unresolved dependency"}</strong>
          {#if node.versionNumber}
            <span class={`dependency-version ${isLowerVersion ? "dependency-version-lower" : ""}`}>
              {node.versionNumber}
            </span>
          {/if}
        </span>
        <span class="dependency-meta">{node.raw}</span>
        {#if helperText}
          <span class="dependency-note">{helperText}</span>
        {/if}
      </div>
    {/if}

    <div class="dependency-side">
      {#if node.effectiveStatus}
        <StatusPill compact={true} status={node.effectiveStatus} />
      {/if}
      {#if node.resolution !== "resolved"}
        <span class={`dependency-state-pill ${stateClass}`}>{stateLabel}</span>
      {/if}
    </div>
  </div>

  {#if hasChildren && isExpanded}
    <div class="dependency-children">
      {#each node.children as child}
        <DependencyTreeNode
          node={child}
          depth={depth + 1}
          {onJumpToDependency}
          {highestVersionByPackage}
        />
      {/each}
    </div>
  {/if}
</div>
