<script lang="ts">
  import { onMount } from "svelte";
  import BrowseScreen from "./components/BrowseScreen.svelte";
  import ClearUnreferencedCacheModal from "./components/ClearUnreferencedCacheModal.svelte";
  import DependencyModal from "./components/DependencyModal.svelte";
  import DownloadsScreen from "./components/DownloadsScreen.svelte";
  import Icon from "./components/Icon.svelte";
  import InstallWarningModal from "./components/InstallWarningModal.svelte";
  import MemoryDiagnosticsModal from "./components/MemoryDiagnosticsModal.svelte";
  import NavRail from "./components/NavRail.svelte";
  import OverviewScreen from "./components/OverviewScreen.svelte";
  import ProfilesScreen from "./components/ProfilesScreen.svelte";
  import ResetProgressModal from "./components/ResetProgressModal.svelte";
  import SettingsScreen from "./components/SettingsScreen.svelte";
  import UninstallDependantsModal from "./components/UninstallDependantsModal.svelte";
  import { actions, appState, selectedProfile } from "./lib/store";
  import type { AppView, EffectiveStatus } from "./lib/types";

  $: focusedVersionId =
    $appState.focusedVersion &&
    $appState.focusedVersion.packageId === $appState.selectedPackageDetail?.id
      ? $appState.focusedVersion.versionId
      : undefined;
  $: focusedVersionToken = $appState.focusedVersion?.highlightToken ?? 0;
  $: activeLaunchMode = $selectedProfile?.launchModeDefault ?? "steam";
  $: launchModeSuffix = activeLaunchMode === "direct" ? " (Direct)" : "";
  $: launchingModded = $appState.isLaunching && $appState.launchingVariant === "modded";
  $: launchingVanilla = $appState.isLaunching && $appState.launchingVariant === "vanilla";

  $: modalCopy =
    $appState.modal
      ? $appState.modal.status === "broken"
        ? {
            title: `This version is marked as broken`,
            description: `${$appState.modal.packageName} ${$appState.modal.versionNumber} is flagged as broken for v49. You can still install it, but this exact build likely has known issues.`,
            note: undefined //$appState.modal.referenceNote
          }
        : {
            title: `This version may be incompatible`,
            description: `${$appState.modal.packageName} ${$appState.modal.versionNumber} was released on or after April 13, 2024, the release of v50, so this mod may be incompatible with v49. You can still continue if you want to experiment.`,
            note: undefined
          }
      : null;

  function setView(view: AppView) {
    actions.setView(view);
  }

  function toggleStatus(status: EffectiveStatus) {
    actions.toggleVisibleStatus(status);
  }

  function preventContextMenu(event: MouseEvent) {
    event.preventDefault();
  }

  onMount(() => {
    window.addEventListener("contextmenu", preventContextMenu);
    void actions.bootstrap();

    return () => {
      window.removeEventListener("contextmenu", preventContextMenu);
    };
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
            on:change={(event) => void actions.selectProfile((event.currentTarget as HTMLSelectElement).value)}
          >
            {#each $appState.profiles as profile}
              <option value={profile.id}>{profile.name}</option>
            {/each}
          </select>
        </label>

        <div class="launch-actions">
          <button
            class="solid-button icon-button"
            type="button"
            disabled={$appState.isBootstrapping || $appState.isLaunching || !$selectedProfile}
            on:click={() => void actions.launchModded()}
          >
            <Icon
              label="Launch modded"
              name={launchingModded ? "refresh" : "play"}
              forceWhite={true}
              spinning={launchingModded}
            />
            <span>{launchingModded ? "Launching..." : `Launch modded${launchModeSuffix}`}</span>
          </button>
          <button
            class="ghost-button icon-button"
            type="button"
            disabled={$appState.isBootstrapping || $appState.isLaunching}
            on:click={() => void actions.launchVanilla()}
          >
            <Icon
              label="Launch vanilla"
              name={launchingVanilla ? "refresh" : "circle"}
              spinning={launchingVanilla}
            />
            <span>{launchingVanilla ? "Launching..." : `Launch vanilla${launchModeSuffix}`}</span>
          </button>
        </div>
      </div>
    </header>

    <div class="main-content">
      <div class="feedback-stack">
        {#if $appState.resourceSaverActive}
          <section class="panel launch-feedback-panel">
            <div class="compact-heading compact-heading-left">
              <Icon label="Resource saver" name="warning" />
              <h3>Resource saver active</h3>
            </div>
            <p>Heavy Browse and reference data loading is paused while Lethal Company is running.</p>
          </section>
        {/if}

        {#if $appState.desktopError}
          <section class="panel desktop-error-panel">
            <div class="compact-heading compact-heading-left">
              <Icon label="Warning" name="warning" />
              <h3>Desktop backend error</h3>
            </div>
            <p>{$appState.desktopError}</p>
          </section>
        {/if}

        {#if $appState.launchFeedback}
          <section class="panel launch-feedback-panel" class:warning={$appState.launchFeedback.tone === "warning"} class:positive={$appState.launchFeedback.tone === "positive"}>
            <div class="compact-heading compact-heading-left">
              <Icon
                label="Launch feedback"
                name={$appState.launchFeedback.tone === "warning" ? "warning" : "check"}
              />
              <h3>{$appState.launchFeedback.title}</h3>
            </div>
            <p>{$appState.launchFeedback.detail}</p>
            <div class="launch-feedback-actions">
              {#if $appState.launchFeedback.diagnosticsPath}
                <button
                  class="ghost-button icon-button"
                  type="button"
                  on:click={() => void actions.openLaunchDiagnostics($appState.launchFeedback?.diagnosticsPath)}
                >
                  <Icon label="Open diagnostics" name="external-link" />
                  <span>Open diagnostics</span>
                </button>
              {/if}

              {#if $appState.launchFeedback.canRepair}
                <button
                  class="ghost-button icon-button"
                  type="button"
                  disabled={$appState.isLaunching}
                  on:click={() => void actions.repairLaunchActivation()}
                >
                  <Icon label="Repair activation" name="refresh" spinning={$appState.isLaunching} />
                  <span>{$appState.isLaunching ? "Repairing..." : "Repair activation"}</span>
                </button>
              {/if}

              <button
                class="ghost-button"
                type="button"
                on:click={() => actions.dismissLaunchFeedback()}
              >
                Dismiss
              </button>
            </div>
          </section>
        {/if}
      </div>

      {#if $appState.view === "overview"}
        <OverviewScreen
          activeProfile={$selectedProfile}
          lastCatalogRefreshLabel={$appState.lastCatalogRefreshLabel}
          onJumpToInstalledModDetails={actions.jumpToInstalledModDetails}
          onToggleInstalledMod={actions.toggleInstalledMod}
          onUninstallInstalledMod={actions.uninstallInstalledMod}
        />
      {:else if $appState.view === "browse"}
        <BrowseScreen
          cards={$appState.catalogCards}
          catalogError={$appState.catalogError}
          {focusedVersionId}
          {focusedVersionToken}
          hasMore={$appState.catalogHasMore}
          isLoadingFirstPage={$appState.isLoadingCatalogFirstPage}
          isLoadingNextPage={$appState.isLoadingCatalogNextPage}
          isRefreshingCatalog={$appState.isRefreshingCatalog}
          onInstall={actions.requestInstall}
          onSwitchVersion={actions.requestSwitchVersion}
          onLoadMore={actions.loadMoreCatalog}
          onRefresh={actions.refreshCatalog}
          onSelectPackage={actions.selectPackage}
          onUninstallPackage={actions.uninstallPackageFromBrowse}
          onUninstallVersion={actions.uninstallVersionFromBrowse}
          onSetReference={actions.setReferenceState}
          onViewDependencies={actions.openDependencyModal}
          onSearchDraftChange={actions.setBrowseSearchDraft}
          onSubmitSearch={actions.submitBrowseSearch}
          onBrowseSortChange={actions.setBrowseSortMode}
          onToggleStatus={toggleStatus}
          browseSortMode={$appState.browseSortMode}
          refreshLabel={$appState.lastCatalogRefreshLabel}
          searchDraft={$appState.browseSearchDraft}
          busyPackageIds={$appState.busyPackageIds}
          isResourceSaverActive={$appState.resourceSaverActive}
          selectedPackage={$appState.selectedPackageDetail}
          installedMods={$selectedProfile?.installedMods ?? []}
          visibleStatuses={$appState.visibleStatuses}
        />
      {:else if $appState.view === "profiles"}
        <ProfilesScreen
          onCreateProfile={actions.createProfile}
          onDeleteSelectedProfile={actions.deleteSelectedProfile}
          onSelectProfile={actions.selectProfile}
          onUpdateProfile={actions.updateProfile}
          profileError={$appState.profileError}
          profiles={$appState.profiles}
          selectedProfile={$selectedProfile}
        />
      {:else if $appState.view === "downloads"}
        <DownloadsScreen downloads={$appState.downloads} />
      {:else if $appState.view === "settings"}
        <SettingsScreen
          cacheSummary={$appState.cacheSummary}
          isLoadingProfilesStorageSummary={$appState.isLoadingProfilesStorageSummary}
          protonRuntimes={$appState.protonRuntimes}
          selectedProtonRuntimeId={$appState.selectedProtonRuntimeId}
          isLoadingProtonRuntimes={$appState.isLoadingProtonRuntimes}
          profilesStorageSummary={$appState.profilesStorageSummary}
          onOpenActiveProfileFolder={actions.openActiveProfileFolder}
          onOpenMemoryDiagnostics={actions.openMemoryDiagnosticsModal}
          onClearCache={actions.clearCache}
          onClearUnreferencedCache={actions.requestClearUnreferencedCache}
          onOpenCacheFolder={actions.openCacheFolder}
          onOpenProfilesFolder={actions.openProfilesFolder}
          onResetAllData={actions.resetAllData}
          onSelectProtonRuntime={actions.selectProtonRuntime}
          onWarningPrefChange={actions.setWarningPreference}
          settingsError={$appState.settingsError}
          warningPrefs={$appState.warningPrefs}
        />
      {/if}
    </div>
  </main>
</div>

{#if $appState.isCatalogOverlayVisible}
  <div class="app-loading-overlay">
    <div class="loading-card panel">
      <div class="loading-spinner" aria-hidden="true"></div>
      <h3>{$appState.catalogOverlayTitle ?? "Retrieving Thunderstore catalog"}</h3>
      <p>{$appState.catalogOverlayMessage ?? "Building local cache for Browse"}</p>
      <div class="loading-steps" aria-label="Catalog refresh progress">
        <div
          class:active={$appState.catalogOverlayStep === "network"}
          class:done={
            $appState.catalogOverlayStep === "cache" ||
            $appState.catalogOverlayStep === "browse" ||
            $appState.catalogOverlayStep === "dependencies"
          }
          class="loading-step"
        >
          <span class="loading-step-dot"></span>
          <span>Contact Thunderstore</span>
        </div>
        <div
          class:active={$appState.catalogOverlayStep === "cache"}
          class:done={$appState.catalogOverlayStep === "browse" || $appState.catalogOverlayStep === "dependencies"}
          class="loading-step"
        >
          <span class="loading-step-dot"></span>
          <span>Update local cache</span>
        </div>
        <div
          class:active={$appState.catalogOverlayStep === "browse"}
          class:done={$appState.catalogOverlayStep === "dependencies"}
          class="loading-step"
        >
          <span class="loading-step-dot"></span>
          <span>Load Browse results</span>
        </div>
        <div class:active={$appState.catalogOverlayStep === "dependencies"} class="loading-step">
          <span class="loading-step-dot"></span>
          <span>Prepare dependencies</span>
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

{#if $appState.uninstallDependantsModal}
  <UninstallDependantsModal
    packageName={$appState.uninstallDependantsModal.packageName}
    dependants={$appState.uninstallDependantsModal.dependants}
    onCancel={actions.dismissUninstallDependantsModal}
    onConfirm={actions.confirmUninstallDependantsModal}
  />
{/if}

{#if $appState.clearUnreferencedCacheModal}
  <ClearUnreferencedCacheModal
    preview={$appState.clearUnreferencedCacheModal}
    onCancel={actions.dismissClearUnreferencedCacheModal}
    onConfirm={actions.confirmClearUnreferencedCacheModal}
  />
{/if}

{#if $appState.memoryDiagnosticsModal}
  <MemoryDiagnosticsModal
    state={$appState.memoryDiagnosticsModal}
    onClose={actions.dismissMemoryDiagnosticsModal}
    onRefresh={actions.refreshMemoryDiagnostics}
  />
{/if}

{#if $appState.resetProgress}
  <ResetProgressModal state={$appState.resetProgress} />
{/if}

{#if $appState.dependencyModal}
  <DependencyModal
    state={$appState.dependencyModal}
    onClose={actions.closeDependencyModal}
    onJumpToDependency={actions.jumpToDependency}
  />
{/if}
