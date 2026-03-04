<script lang="ts">
  import type { AppView } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let activeView: AppView;
  export let onSelect: (view: AppView) => void;

  const items: Array<{ id: AppView; label: string; icon: "home" | "search" | "profiles" | "download" | "settings" }> = [
    { id: "overview", label: "Overview", icon: "home" },
    { id: "browse", label: "Browse", icon: "search" },
    { id: "profiles", label: "Profiles", icon: "profiles" },
    { id: "downloads", label: "Downloads", icon: "download" },
    { id: "settings", label: "Settings", icon: "settings" }
  ];
</script>

<aside class="nav-rail panel">
  <div class="brand-block">
    <h1>49modman</h1>
    <p class="brand-copy">Lethal Company v49</p>
  </div>

  <nav class="nav-list">
    {#each items as item}
      <button
        class:active={item.id === activeView}
        class="nav-button"
        type="button"
        on:click={() => onSelect(item.id)}
      >
        <Icon label={item.label} name={item.icon} />
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>
</aside>
