<script lang="ts">
  import type { OnboardingMode, SteamScanResult, V49ValidationResult } from "../lib/types";
  import Icon from "./Icon.svelte";

  const depotCommand = "download_depot 1966720 1966721 7525563530173177311";

  export let mode: OnboardingMode = null;
  export let pathDraft = "";
  export let scan: SteamScanResult | undefined;
  export let validation: V49ValidationResult | undefined;
  export let isDetecting = false;
  export let isPicking = false;
  export let isValidating = false;
  export let error: string | null = null;
  export let onPathDraftChange: (path: string) => void | Promise<void>;
  export let onDetect: () => void | Promise<void>;
  export let onPickFolder: () => void | Promise<void>;
  export let onValidate: () => void | Promise<void>;
  export let onExit: () => void | Promise<void>;
  export let onOpenSteamConsole: () => void | Promise<void>;
  export let onCopyDepotCommand: () => void | Promise<void>;

  function handleDraftInput(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value;
    void onPathDraftChange(value);
  }

  function guidanceForCode(code: string | undefined) {
    switch (code) {
      case "GAME_PATH_RESOLUTION_FAILED":
        return "Choose the exact Lethal Company install folder manually, then run validation again.";
      case "GAME_EXECUTABLE_MISSING":
      case "GAME_DATA_DIR_MISSING":
        return "The selected folder is not a valid game root. Pick the folder that contains Lethal Company.exe.";
      case "V49_SIGNATURE_MISMATCH":
      case "V49_SIGNATURE_UNCONFIGURED":
        return "Install does not match supported v49 yet. Follow the depot workflow and re-check.";
      case "GAME_PATH_NOT_WRITABLE":
        return "49modman must be able to write to this folder for activation. Check permissions and ownership.";
      case "OK":
        return mode === "required"
          ? "Validation passed. Onboarding completion was saved and full startup will continue."
          : "Validation passed. Onboarding completion was refreshed.";
      default:
        return "Review failed checks below, adjust path/setup, then re-check.";
    }
  }

  $: validationCode = validation?.code;
  $: showDepotWorkflow =
    validationCode === "V49_SIGNATURE_MISMATCH" || validationCode === "V49_SIGNATURE_UNCONFIGURED";
  $: guidance = guidanceForCode(validationCode);
</script>

<section class="screen-stack onboarding-screen">
  <div class="panel simple-hero compact-hero">
    <div>
      <h2>Initial setup</h2>
      <p>Find and validate your Lethal Company install before continuing.</p>
    </div>
    <div class="hero-inline">
      <span class="inline-label">v49 onboarding</span>
    </div>
  </div>

  <section class="panel list-panel">
    <div class="compact-heading compact-heading-left">
      <div class="section-title-row">
        <Icon label="Game install" name="folder" />
        <h2>Game install path</h2>
      </div>
      <div class="section-actions">
        {#if mode === "manual"}
          <button class="ghost-button" type="button" on:click={() => void onExit()} disabled={isDetecting || isPicking || isValidating}>
            Back to Settings
          </button>
        {/if}
        <button class="ghost-button icon-button" type="button" on:click={() => void onDetect()} disabled={isDetecting || isPicking || isValidating}>
          <Icon label="Detect install" name={isDetecting ? "refresh" : "search"} spinning={isDetecting} />
          <span>{isDetecting ? "Detecting..." : "Detect again"}</span>
        </button>
      </div>
    </div>

    <div class="profile-form-card">
      <div class="profile-form-grid">
        <label class="form-field form-field-wide">
          <span>Path</span>
          <input
            value={pathDraft}
            type="text"
            on:input={handleDraftInput}
            placeholder="Select your Lethal Company folder"
            disabled={isValidating}
          />
        </label>
      </div>

      <div class="form-actions form-actions-end">
        <button class="solid-button icon-button" type="button" on:click={() => void onPickFolder()} disabled={isDetecting || isPicking || isValidating}>
          <Icon label="Browse folders" name={isPicking ? "refresh" : "folder"} spinning={isPicking} forceWhite={true} />
          <span>{isPicking ? "Opening..." : "Browse..."}</span>
        </button>
      </div>
    </div>

    <div class="onboarding-detect-state">
      {#if isDetecting}
        <p class="empty-copy">Detecting Steam libraries and game folders...</p>
      {:else if scan?.selectedGamePath}
        <p class="empty-copy">Detected path: <strong>{scan.selectedGamePath}</strong></p>
      {:else}
        <p class="empty-copy">No install path detected yet. Use Browse to pick the folder manually.</p>
      {/if}
    </div>

    {#if scan && scan.gamePaths.length > 0}
      <div class="onboarding-detected-list">
        <h3>Detected candidates</h3>
        <div class="reference-table">
          {#each scan.gamePaths as gamePath}
            <button
              class="ghost-button"
              type="button"
              on:click={() => void onPathDraftChange(gamePath)}
              disabled={isDetecting || isPicking || isValidating}
            >
              {gamePath}
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <div class="form-actions form-actions-end onboarding-validate-actions">
      <button class="solid-button icon-button" type="button" on:click={() => void onValidate()} disabled={isDetecting || isPicking || isValidating}>
        <Icon label="Validate install" name={isValidating ? "refresh" : "check"} spinning={isValidating} forceWhite={true} />
        <span>{isValidating ? "Validating..." : validation ? "Re-check" : mode === "required" ? "Validate and continue" : "Validate"}</span>
      </button>
    </div>

    {#if validation}
      <div class="onboarding-validation-block">
        <div class="compact-heading compact-heading-left">
          <Icon label="Validation" name={validation.ok ? "check" : "warning"} />
          <h3>{validation.ok ? "Validation passed" : "Validation failed"}</h3>
        </div>
        <p class="empty-copy">{validation.message}</p>
        <p class="warning-copy">{guidance}</p>

        {#if validation.resolvedGamePath}
          <p class="empty-copy">Resolved path: <strong>{validation.resolvedGamePath}</strong></p>
        {/if}

        <div class="reference-table">
          {#each validation.checks as check}
            <div class="onboarding-check-row">
              <div class="section-title-row">
                <Icon label={check.ok ? "Passed" : "Failed"} name={check.ok ? "check" : "warning"} />
                <strong>{check.code}</strong>
              </div>
              <p class="empty-copy">{check.message}</p>
              {#if check.detail}
                <p class="warning-copy">{check.detail}</p>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}

    {#if showDepotWorkflow}
      <div class="onboarding-depot panel compact-panel">
        <div class="compact-heading compact-heading-left">
          <Icon label="Depot setup" name="warning" />
          <h3>Manual depot workflow</h3>
        </div>
        <ol>
          <li>Open Steam console.</li>
          <li>Run the depot command shown below.</li>
          <li>Copy depot files into your selected game folder.</li>
          <li>Return here and press Re-check.</li>
        </ol>
        <pre>{depotCommand}</pre>
        <div class="form-actions form-actions-end">
          <button class="ghost-button icon-button" type="button" on:click={() => void onOpenSteamConsole()} disabled={isValidating}>
            <Icon label="Open Steam console" name="external-link" />
            <span>Open Steam console</span>
          </button>
          <button class="ghost-button icon-button" type="button" on:click={() => void onCopyDepotCommand()} disabled={isValidating}>
            <Icon label="Copy depot command" name="upload" />
            <span>Copy command</span>
          </button>
        </div>
      </div>
    {/if}

    {#if error}
      <p class="warning-copy danger">{error}</p>
    {/if}
  </section>
</section>
