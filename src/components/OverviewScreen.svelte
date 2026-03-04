<script lang="ts">
  import type { ProfileDetailDto } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let activeProfile: ProfileDetailDto | undefined;
  export let lastCatalogRefreshLabel: string;
</script>

<section class="screen-stack overview-screen">
  <div class="panel simple-hero compact-hero">
    <div>
      <h2>{activeProfile?.name ?? "No active profile selected"}</h2>
      <p>{activeProfile?.notes ?? "Pick a profile to see its current defaults."}</p>
    </div>
    <div class="hero-inline">
      <span class="inline-label">{activeProfile?.launchModeDefault ?? "n/a"} launch</span>
      <span class="inline-dot">•</span>
      <span class="inline-label">{lastCatalogRefreshLabel}</span>
    </div>
  </div>

  <section class="panel list-panel">
    <div class="compact-heading compact-heading-left">
      <Icon label="Installed mods" name="details" />
      <h3>Installed mods</h3>
    </div>

    <div class="installed-list list-scroll">
      <p class="empty-copy">No mods installed yet.</p>
    </div>
  </section>
</section>
