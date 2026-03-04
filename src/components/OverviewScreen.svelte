<script lang="ts">
  import type { ProfileDetailDto } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let activeProfile: ProfileDetailDto | undefined;
  export let lastCatalogRefreshLabel: string;

  function formatInstalledAt(value: string) {
    const parsed = Date.parse(value);
    if (Number.isNaN(parsed)) {
      return value;
    }

    return new Date(parsed).toLocaleString();
  }
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
      {#if !activeProfile || activeProfile.installedMods.length === 0}
        <p class="empty-copy">No mods installed yet.</p>
      {:else}
        {#each activeProfile.installedMods as mod}
          <article class="installed-card">
            <div class="installed-card-main">
              <div class="installed-card-icon-wrap">
                {#if mod.iconDataUrl}
                  <img alt={`${mod.packageName} icon`} class="installed-card-icon" src={mod.iconDataUrl} />
                {:else}
                  <div aria-hidden="true" class="installed-card-icon installed-card-icon-fallback">
                    <span>{mod.packageName.slice(0, 1).toUpperCase()}</span>
                  </div>
                {/if}
              </div>
              <div class="installed-card-copy">
                <strong>{mod.packageName}</strong>
                <p>
                  {mod.versionNumber} · {mod.enabled ? "enabled" : "disabled"}
                </p>
                <p>Installed {formatInstalledAt(mod.installedAt)}</p>
              </div>
            </div>
          </article>
        {/each}
      {/if}
    </div>
  </section>
</section>
