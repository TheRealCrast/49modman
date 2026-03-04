<script lang="ts">
  import { onDestroy, tick } from "svelte";
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
  export let onViewDependencies: (request: {
    packageId: string;
    packageName: string;
    versionId: string;
    versionNumber: string;
  }) => void;
  export let focusedVersionId: string | undefined = undefined;
  export let focusedVersionToken = 0;
  export let isLocked = false;

  const filters = ["verified", "green", "yellow", "orange", "red", "broken"] as const;
  const versionRowElements = new Map<string, HTMLElement>();
  let installVersion: ModVersion | undefined = undefined;
  let installStatus: EffectiveStatus = "green";
  let installLabel = "Install";
  let menu:
    | {
        versionId: string;
        versionNumber: string;
        x: number;
        y: number;
      }
    | null = null;
  let lastFocusedKey = "";

  function openMenuForVersion(
    version: ModVersion,
    x: number,
    y: number
  ) {
    if (isLocked) {
      return;
    }

    menu = {
      versionId: version.id,
      versionNumber: version.versionNumber,
      x: Math.max(16, Math.min(window.innerWidth - 236, x)),
      y: Math.max(16, Math.min(window.innerHeight - 220, y))
    };
  }

  function openContextMenu(event: MouseEvent, version: ModVersion) {
    if (isLocked) {
      return;
    }

    event.preventDefault();
    openMenuForVersion(version, event.clientX, event.clientY);
  }

  function openOverflowMenu(event: MouseEvent, version: ModVersion) {
    if (isLocked) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    const button = event.currentTarget as HTMLButtonElement;
    const rect = button.getBoundingClientRect();
    openMenuForVersion(version, rect.left - 188, rect.bottom + 8);
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

  function registerVersionRow(node: HTMLElement, versionId: string) {
    versionRowElements.set(versionId, node);

    return {
      destroy() {
        versionRowElements.delete(versionId);
      }
    };
  }

  function viewDependenciesForMenuVersion() {
    if (isLocked) {
      return;
    }

    if (!pkg || !menu) {
      return;
    }

    onViewDependencies({
      packageId: pkg.id,
      packageName: pkg.fullName,
      versionId: menu.versionId,
      versionNumber: menu.versionNumber
    });
    closeMenu();
  }

  async function viewPackageInBrowser() {
    if (isLocked) {
      return;
    }

    if (!pkg?.websiteUrl) {
      return;
    }

    await openExternalUrl(pkg.websiteUrl);
    closeMenu();
  }

  function installRecommendedVersion() {
    if (isLocked) {
      return;
    }

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
    if (isLocked) {
      return;
    }

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

  $: if (isLocked && menu) {
    closeMenu();
  }

  async function scrollFocusedVersionIntoView(versionId: string) {
    await tick();
    versionRowElements.get(versionId)?.scrollIntoView({
      behavior: "smooth",
      block: "center"
    });
  }

  $: {
    const focusKey =
      pkg && focusedVersionId
        ? `${pkg.id}:${focusedVersionId}:${focusedVersionToken}`
        : "";

    if (focusKey && focusKey !== lastFocusedKey) {
      lastFocusedKey = focusKey;
      void scrollFocusedVersionIntoView(focusedVersionId!);
    }
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
          disabled={isLocked}
          on:click={installRecommendedVersion}
        >
          <Icon
            label={`Install ${installVersion?.versionNumber ?? "recommended version"}`}
            name="download"
            forceWhite={true}
          />
          <span>{installLabel}</span>
        </button>
      {/key}
      <button
        class="ghost-button icon-button detail-link-button"
        type="button"
        disabled={isLocked}
        on:click={viewPackageInBrowser}
      >
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
          <article
            use:registerVersionRow={version.id}
            class:focused-version={focusedVersionId === version.id}
            class="version-row"
            on:contextmenu={(event) => openContextMenu(event, version)}
          >
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
                disabled={isLocked}
                on:click={() => {
                  if (isLocked) {
                    return;
                  }
                  const request = buildInstallRequest(version.id);
                  if (request) {
                    onInstall(request);
                  }
                }}
              >
                <Icon label="Install version" name="download" forceWhite={true} />
                <span>Install version</span>
              </button>
              <button
                aria-expanded={menu?.versionId === version.id}
                class="ghost-button icon-button version-menu-trigger"
                type="button"
                disabled={isLocked}
                on:click={(event) => openOverflowMenu(event, version)}
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
        <button class="version-menu-item" type="button" on:click={viewDependenciesForMenuVersion}>
          <Icon label="View dependencies" name="details" size={16} />
          <span>View dependencies</span>
        </button>
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

    {#if isLocked}
      <div class="detail-lock-overlay" aria-live="polite">
        <div class="detail-lock-card">
          <div class="loading-spinner" aria-hidden="true"></div>
          <p>Searching cached mods...</p>
        </div>
      </div>
    {/if}
  </section>
{:else}
  <section class="panel detail-panel empty-panel">
    <h3>Select a mod</h3>
    <p>Choose a package from the browser to inspect its versions, local references, and install warnings.</p>
  </section>
{/if}
