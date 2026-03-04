<script lang="ts">
  import { onMount } from "svelte";
  import BrowseScreen from "./components/BrowseScreen.svelte";
  import DownloadsScreen from "./components/DownloadsScreen.svelte";
  import Icon from "./components/Icon.svelte";
  import InstallWarningModal from "./components/InstallWarningModal.svelte";
  import NavRail from "./components/NavRail.svelte";
  import OverviewScreen from "./components/OverviewScreen.svelte";
  import ProfilesScreen from "./components/ProfilesScreen.svelte";
  import SettingsScreen from "./components/SettingsScreen.svelte";
  import { actions, appState, selectedProfile } from "./lib/store";
  import type { AppView, EffectiveStatus } from "./lib/types";

  $: modalVersion =
    $appState.modal &&
    $appState.packages
      .find((pkg) => pkg.id === $appState.modal?.packageId)
      ?.versions.find((version) => version.id === $appState.modal?.versionId);

  $: modalPackage = $appState.modal && $appState.packages.find((pkg) => pkg.id === $appState.modal?.packageId);

  $: modalCopy =
    modalVersion && $appState.modal
      ? $appState.modal.status === "broken"
        ? {
            title: `This version is marked broken locally`,
            description: `${modalPackage?.fullName} ${modalVersion.versionNumber} is flagged as broken for v49 in your local reference library. You can still install it, but the UI is warning you because this exact build has known issues.`,
            note:
              modalVersion.overrideReferenceNote ??
              modalVersion.bundledReferenceNote
          }
        : {
            title: `This version falls in the red zone`,
            description: `${modalPackage?.fullName} ${modalVersion.versionNumber} was released on or after April 13, 2024, so the frontend treats it as incompatible with the v49 target window. You can still continue if you want to experiment.`,
            note: undefined
          }
      : null;

  function setView(view: AppView) {
    actions.setView(view);
  }

  function toggleStatus(status: EffectiveStatus) {
    actions.toggleVisibleStatus(status);
  }

  onMount(() => {
    void actions.bootstrap();
  });
</script>

<svelte:head>
  <title>49modman</title>
  <meta
    content="Lightweight desktop mod manager for Lethal Company v49."
    name="description"
  />
</svelte:head>

<div class="app-shell">
  <NavRail activeView={$appState.view} onSelect={setView} />

  <main class="main-shell">
    <header class="topbar panel">
      <div class="topbar-copy">
        <h2>49modman</h2>
        <p>Lethal Company v49</p>
      </div>

      <div class="topbar-actions">
        <label class="profile-select">
          <span>Profile</span>
          <select
            value={$appState.selectedProfileId}
            on:change={(event) => actions.selectProfile((event.currentTarget as HTMLSelectElement).value)}
          >
            {#each $appState.profiles as profile}
              <option value={profile.id}>{profile.name}</option>
            {/each}
          </select>
        </label>

        <div class="launch-actions">
          <button class="solid-button icon-button" type="button">
            <Icon label="Launch modded" name="play" />
            <span>Launch modded</span>
          </button>
          <button class="ghost-button icon-button" type="button">
            <Icon label="Launch vanilla" name="circle" />
            <span>Launch vanilla</span>
          </button>
        </div>
      </div>
    </header>

    {#if $appState.desktopError}
      <section class="panel desktop-error-panel">
        <div class="compact-heading compact-heading-left">
          <Icon label="Warning" name="warning" />
          <h3>Desktop backend error</h3>
        </div>
        <p>{$appState.desktopError}</p>
      </section>
    {/if}

    {#if $appState.view === "overview"}
      <OverviewScreen
        activeProfile={$selectedProfile}
        packages={$appState.packages}
        lastCatalogRefreshLabel={$appState.lastCatalogRefreshLabel}
        onToggleMod={actions.toggleInstalledMod}
        onUninstallMod={actions.uninstallInstalledMod}
      />
    {:else if $appState.view === "browse"}
      <BrowseScreen
        cards={$appState.catalogCards}
        catalogError={$appState.catalogError}
        hasMore={$appState.catalogHasMore}
        isLoadingFirstPage={$appState.isLoadingCatalogFirstPage}
        isLoadingNextPage={$appState.isLoadingCatalogNextPage}
        isRefreshingCatalog={$appState.isRefreshingCatalog}
        onInstall={actions.requestInstall}
        onLoadMore={actions.loadMoreCatalog}
        onRefresh={actions.refreshCatalog}
        onSelectPackage={actions.selectPackage}
        onSetReference={actions.setReferenceState}
        onSearchDraftChange={actions.setBrowseSearchDraft}
        onSubmitSearch={actions.submitBrowseSearch}
        onToggleStatus={toggleStatus}
        refreshLabel={$appState.lastCatalogRefreshLabel}
        searchDraft={$appState.browseSearchDraft}
        selectedPackage={$appState.selectedPackageDetail}
        visibleStatuses={$appState.visibleStatuses}
      />
    {:else if $appState.view === "profiles"}
      <ProfilesScreen
        onSelectProfile={actions.selectProfile}
        profiles={$appState.profiles}
        selectedProfile={$selectedProfile}
      />
    {:else if $appState.view === "downloads"}
      <DownloadsScreen downloads={$appState.downloads} />
    {:else if $appState.view === "settings"}
      <SettingsScreen
        onWarningPrefChange={actions.setWarningPreference}
        warningPrefs={$appState.warningPrefs}
      />
    {/if}
  </main>
</div>

{#if $appState.isCatalogOverlayVisible}
  <div class="app-loading-overlay">
    <div class="loading-card panel">
      <div class="loading-spinner" aria-hidden="true"></div>
      <h3>{$appState.catalogOverlayTitle ?? "Retrieving Thunderstore catalog"}</h3>
      <p>{$appState.catalogOverlayMessage ?? "Building local cache for Browse"}</p>
      <div class="loading-steps" aria-label="Catalog refresh progress">
        <div class:active={$appState.catalogOverlayStep === "network"} class:done={$appState.catalogOverlayStep !== "network" && $appState.catalogOverlayStep !== null} class="loading-step">
          <span class="loading-step-dot"></span>
          <span>Contact Thunderstore</span>
        </div>
        <div class:active={$appState.catalogOverlayStep === "cache"} class:done={$appState.catalogOverlayStep === "browse"} class="loading-step">
          <span class="loading-step-dot"></span>
          <span>Update local cache</span>
        </div>
        <div class:active={$appState.catalogOverlayStep === "browse"} class="loading-step">
          <span class="loading-step-dot"></span>
          <span>Load Browse results</span>
        </div>
      </div>
    </div>
  </div>
{/if}

{#if $appState.modal && modalCopy}
  <InstallWarningModal
    description={modalCopy.description}
    note={modalCopy.note}
    onCancel={actions.dismissModal}
    onConfirm={actions.confirmModal}
    title={modalCopy.title}
  />
{/if}
