<script lang="ts">
  import type { ModPackage, Profile } from "../lib/types";
  import Icon from "./Icon.svelte";
  import StatusPill from "./StatusPill.svelte";
  import { resolveEffectiveStatus } from "../lib/status";

  export let activeProfile: Profile | undefined;
  export let packages: ModPackage[] = [];
  export let lastCatalogRefreshLabel: string;
  export let onToggleMod: (packageId: string, versionId: string) => void;
  export let onUninstallMod: (packageId: string, versionId: string) => void;

  function packageName(packageId: string): string {
    return packages.find((pkg) => pkg.id === packageId)?.fullName ?? packageId;
  }

  function versionNumber(packageId: string, versionId: string): string {
    return (
      packages.find((pkg) => pkg.id === packageId)?.versions.find((entry) => entry.id === versionId)
        ?.versionNumber ?? "unknown"
    );
  }

  function versionStatus(packageId: string, versionId: string) {
    const version = packages
      .find((pkg) => pkg.id === packageId)
      ?.versions.find((entry) => entry.id === versionId);

    return version ? resolveEffectiveStatus(version) : "orange";
  }
</script>

<section class="screen-stack overview-screen">
  <div class="panel simple-hero compact-hero">
    <div>
      <h2>{activeProfile?.name ?? "No active profile selected"}</h2>
      <p>{activeProfile?.notes ?? "Pick a profile to see its installed mods and current launch defaults."}</p>
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

    {#if activeProfile}
      <div class="installed-list list-scroll">
        {#if activeProfile.installedMods.length === 0}
          <p class="empty-copy">This profile is currently clean.</p>
        {/if}

        {#each activeProfile.installedMods as mod}
          <article class="installed-card">
            <div class="installed-card-main">
              <div>
                <strong>{packageName(mod.packageId)}</strong>
                <p>Version {versionNumber(mod.packageId, mod.versionId)}</p>
              </div>
              <div class="installed-card-actions">
                <button
                  class="ghost-button icon-button toggle-icon-button"
                  type="button"
                  on:click={() => onToggleMod(mod.packageId, mod.versionId)}
                >
                  <Icon
                    label={mod.enabled ? "Disable mod" : "Enable mod"}
                    name={mod.enabled ? "check" : "circle"}
                  />
                  <span>{mod.enabled ? "Enabled" : "Disabled"}</span>
                </button>
                <button
                  class="ghost-button icon-button danger-outline"
                  type="button"
                  on:click={() => onUninstallMod(mod.packageId, mod.versionId)}
                >
                  <Icon label="Uninstall mod" name="x-close" />
                  <span>Uninstall</span>
                </button>
              </div>
            </div>
            <div class="installed-meta">
              <StatusPill compact={true} status={versionStatus(mod.packageId, mod.versionId)} />
              <span>{mod.enabled ? "Enabled" : "Disabled"}</span>
            </div>
          </article>
        {/each}
      </div>
    {:else}
      <p class="empty-copy">Select a profile to inspect its installed versions.</p>
    {/if}
  </section>
</section>
