<script lang="ts">
  import { onDestroy } from "svelte";
  import { openExternalUrl } from "../lib/api/system";
  import { currentReferenceNote, pickRecommendedVersion, resolveEffectiveStatus } from "../lib/status";
  import type { EffectiveStatus, InstallRequest, ModPackage, ModVersion } from "../lib/types";
  import Icon from "./Icon.svelte";
  import StatusPill from "./StatusPill.svelte";

  export let pkg: ModPackage | undefined;
  export let visibleStatuses: string[];
  export let onToggleStatus: (status: "verified" | "broken" | "green" | "yellow" | "orange" | "red") => void;
  export let onInstall: (request: InstallRequest) => void;
  export let onSetReference: (packageId: string, versionId: string, state: "verified" | "broken" | "neutral") => void;

  const filters = ["verified", "green", "yellow", "orange", "red", "broken"] as const;
  let installVersion: ModVersion | undefined = undefined;
  let installStatus: EffectiveStatus = "green";
  let installLabel = "Install";
  let menu:
    | {
        versionId: string;
        x: number;
        y: number;
      }
    | null = null;

  function openMenuForVersion(
    versionId: string,
    x: number,
    y: number
  ) {
    menu = {
      versionId,
      x: Math.max(16, Math.min(window.innerWidth - 236, x)),
      y: Math.max(16, Math.min(window.innerHeight - 220, y))
    };
  }

  function openContextMenu(event: MouseEvent, versionId: string) {
    event.preventDefault();
    openMenuForVersion(versionId, event.clientX, event.clientY);
  }

  function openOverflowMenu(event: MouseEvent, versionId: string) {
    event.preventDefault();
    event.stopPropagation();
    const button = event.currentTarget as HTMLButtonElement;
    const rect = button.getBoundingClientRect();
    openMenuForVersion(versionId, rect.left - 188, rect.bottom + 8);
  }

  function closeMenu() {
    menu = null;
  }

  function pickInstallVersion() {
    if (!pkg) {
      return undefined;
    }

    return pickRecommendedVersion(pkg);
  }

  function buildInstallRequest(versionId: string): InstallRequest | undefined {
    if (!pkg) {
      return undefined;
    }

    const version = pkg.versions.find((entry) => entry.id === versionId);
    if (!version) {
      return undefined;
    }

    return {
      packageId: pkg.id,
      packageName: pkg.fullName,
      versionId: version.id,
      versionNumber: version.versionNumber,
      effectiveStatus: version.effectiveStatus ?? resolveEffectiveStatus(version),
      referenceNote: currentReferenceNote(version)
    };
  }

  async function viewPackageInBrowser() {
    if (!pkg?.websiteUrl) {
      return;
    }

    await openExternalUrl(pkg.websiteUrl);
    closeMenu();
  }

  function installRecommendedVersion() {
    if (!pkg) {
      return;
    }

    const targetVersion = pickInstallVersion();

    if (!targetVersion) {
      return;
    }

    const request = buildInstallRequest(targetVersion.id);
    if (!request) {
      return;
    }

    onInstall(request);
  }

  function applyReference(state: "verified" | "broken" | "neutral") {
    if (!pkg || !menu) {
      return;
    }

    onSetReference(pkg.id, menu.versionId, state);
    closeMenu();
  }

  function handleWindowPointerDown(event: PointerEvent) {
    const target = event.target as HTMLElement | null;

    if (target?.closest(".version-menu") || target?.closest(".version-menu-trigger")) {
      return;
    }

    closeMenu();
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      closeMenu();
    }
  }

  onDestroy(() => {
    window.removeEventListener("pointerdown", handleWindowPointerDown);
    window.removeEventListener("keydown", handleWindowKeydown);
  });

  $: if (menu) {
    window.addEventListener("pointerdown", handleWindowPointerDown);
    window.addEventListener("keydown", handleWindowKeydown);
  } else {
    window.removeEventListener("pointerdown", handleWindowPointerDown);
    window.removeEventListener("keydown", handleWindowKeydown);
  }

  $: if (pkg) {
    installVersion = pickRecommendedVersion(pkg);
    installStatus = installVersion.effectiveStatus ?? resolveEffectiveStatus(installVersion);
    installLabel = `Install ${installVersion.versionNumber}`;
  } else {
    installVersion = undefined;
    installStatus = "green";
    installLabel = "Install";
  }
</script>

{#if pkg}
  <section class="panel detail-panel list-panel detail-panel-grid">
    <div class="detail-header detail-header-left">
      <div>
        <h3>{pkg.fullName}</h3>
        <p>{pkg.summary}</p>
      </div>

      <div class="detail-metrics">
        <span>{pkg.versions.length} versions</span>
      </div>
    </div>

    <div class="detail-primary-actions">
      {#key installVersion?.id ?? `${pkg.id}-install`}
        <button
          class={`solid-button icon-button package-install-button ${installStatus}`}
          type="button"
          on:click={installRecommendedVersion}
        >
          <Icon label={`Install ${installVersion?.versionNumber ?? "recommended version"}`} name="download" />
          <span>{installLabel}</span>
        </button>
      {/key}
      <button class="ghost-button icon-button detail-link-button" type="button" on:click={viewPackageInBrowser}>
        <Icon label="View mod in browser" name="external-link" size={16} />
        <span>View in browser</span>
      </button>
    </div>

    <div class="chip-row">
      {#each pkg.categories as category}
        <span class="category-chip">{category}</span>
      {/each}
    </div>

    <div class="filter-row">
      {#each filters as filter}
        <button
          class:active={visibleStatuses.includes(filter)}
          class={`toggle-chip ${filter}`}
          type="button"
          on:click={() => onToggleStatus(filter)}
        >
          {filter}
        </button>
      {/each}
    </div>

    <div class="versions-list list-scroll">
      {#each [...pkg.versions].sort((left, right) => right.publishedAt.localeCompare(left.publishedAt)) as version}
        {#if visibleStatuses.includes(version.effectiveStatus ?? resolveEffectiveStatus(version))}
          <article class="version-row" on:contextmenu={(event) => openContextMenu(event, version.id)}>
            <div class="version-main">
              <div class="version-title">
                <strong>{version.versionNumber}</strong>
                <StatusPill status={version.effectiveStatus ?? resolveEffectiveStatus(version)} />
              </div>
              <p>
                Published {version.publishedAt} · {version.downloads.toLocaleString()} downloads
              </p>

              {#if version.overrideReferenceState === "verified" || version.overrideReferenceState === "broken"}
                <p class="reference-note">
                  Local override: {version.overrideReferenceNote}
                </p>
              {:else if version.bundledReferenceState}
                <p class="reference-note">
                  Bundled reference: {version.bundledReferenceNote}
                </p>
              {/if}
            </div>

            <div class="version-actions">
              <button
                class="solid-button icon-button"
                type="button"
                on:click={() => {
                  const request = buildInstallRequest(version.id);
                  if (request) {
                    onInstall(request);
                  }
                }}
              >
                <Icon label="Install version" name="download" />
                <span>Install version</span>
              </button>
              <button
                aria-expanded={menu?.versionId === version.id}
                class="ghost-button icon-button version-menu-trigger"
                type="button"
                on:click={(event) => openOverflowMenu(event, version.id)}
              >
                <Icon label="Version actions" name="three-dots-vertical" />
              </button>
            </div>
          </article>
        {/if}
      {/each}
    </div>

    {#if menu}
      <div class="version-menu panel" style={`left:${menu.x}px;top:${menu.y}px;`}>
        <button class="version-menu-item" type="button" on:click={() => applyReference("verified")}>
          <Icon label="Mark as verified" name="verified" size={16} />
          <span>Mark as verified</span>
        </button>
        <button class="version-menu-item danger" type="button" on:click={() => applyReference("broken")}>
          <Icon label="Mark as broken" name="broken" size={16} />
          <span>Mark as broken</span>
        </button>
        <button class="version-menu-item" type="button" on:click={() => applyReference("neutral")}>
          <Icon label="Clear mark" name="x-close" size={16} />
          <span>Clear mark</span>
        </button>
      </div>
    {/if}
  </section>
{:else}
  <section class="panel detail-panel empty-panel">
    <h3>Select a mod</h3>
    <p>Choose a package from the browser to inspect its versions, local references, and install warnings.</p>
  </section>
{/if}
