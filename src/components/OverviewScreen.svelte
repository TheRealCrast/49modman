<script lang="ts">
  import type { ProfileDetailDto, ProfileInstalledModDto } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let activeProfile: ProfileDetailDto | undefined;
  export let lastCatalogRefreshLabel: string;
  export let onToggleInstalledMod: (
    profileId: string,
    packageId: string,
    versionId: string,
    enabled: boolean
  ) => void | Promise<void>;
  export let onUninstallInstalledMod: (
    profileId: string,
    packageId: string,
    versionId: string
  ) => void | Promise<void>;
  export let onJumpToInstalledModDetails: (
    packageId: string,
    versionId: string
  ) => void | Promise<void>;

  function formatInstalledAt(value: string) {
    const parsed = Date.parse(value);
    if (Number.isNaN(parsed)) {
      return value;
    }

    return new Date(parsed).toLocaleString();
  }

  async function toggleMod(mod: ProfileInstalledModDto) {
    if (!activeProfile) {
      return;
    }

    await onToggleInstalledMod(activeProfile.id, mod.packageId, mod.versionId, !mod.enabled);
  }

  async function uninstallMod(mod: ProfileInstalledModDto) {
    if (!activeProfile) {
      return;
    }

    await onUninstallInstalledMod(activeProfile.id, mod.packageId, mod.versionId);
  }

  $: installedCount = activeProfile?.installedMods.length ?? 0;
  $: installedModsHeading = `${installedCount} installed mod${installedCount === 1 ? "" : "s"}`;
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
      <Icon label={installedModsHeading} name="details" />
      <h3 class="installed-mods-title">{installedModsHeading}</h3>
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
                <p>{mod.versionNumber}</p>
                <p>Installed {formatInstalledAt(mod.installedAt)}</p>
              </div>
            </div>
            <div class="installed-card-actions">
              <button
                aria-pressed={mod.enabled}
                class="ghost-button icon-button toggle-icon-button"
                type="button"
                on:click={() => void toggleMod(mod)}
              >
                <Icon label={mod.enabled ? "Enabled" : "Disabled"} name={mod.enabled ? "check" : "circle"} />
                <span>{mod.enabled ? "Enabled" : "Disabled"}</span>
              </button>
              <button class="ghost-button danger-outline" type="button" on:click={() => void uninstallMod(mod)}>
                Uninstall
              </button>
              <button class="ghost-button icon-button" type="button" on:click={() => void onJumpToInstalledModDetails(mod.packageId, mod.versionId)}>
                <Icon label="Show details" name="details" size={16} />
                <span>Show details</span>
              </button>
            </div>
          </article>
        {/each}
      {/if}
    </div>
  </section>
</section>
