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
        return "Pick your Lethal Company install folder and try again.";
      case "GAME_EXECUTABLE_MISSING":
      case "GAME_DATA_DIR_MISSING":
        return "That folder does not look like the game install folder yet.";
      case "V49_SIGNATURE_MISMATCH":
      case "V49_SIGNATURE_UNCONFIGURED":
        return "Your game files are not on the supported v49 build yet. Follow the install steps below, then check again.";
      case "GAME_PATH_NOT_WRITABLE":
        return "49modman needs permission to write to this folder.";
      case "OK":
        return mode === "required"
          ? "Everything looks good. You can continue."
          : "Everything looks good.";
      default:
        return "Please adjust the folder or setup and try again.";
    }
  }

  function summaryForValidation(result: V49ValidationResult | undefined) {
    if (!result) {
      return "";
    }

    switch (result.code) {
      case "OK":
        return "Lethal Company was found and is ready for 49modman.";
      case "GAME_PATH_RESOLUTION_FAILED":
        return "We could not find a valid Lethal Company folder from that path.";
      case "GAME_EXECUTABLE_MISSING":
      case "GAME_DATA_DIR_MISSING":
        return "This folder is missing required Lethal Company files.";
      case "V49_SIGNATURE_MISMATCH":
      case "V49_SIGNATURE_UNCONFIGURED":
        return "This install is not on the supported v49 files yet.";
      case "GAME_PATH_NOT_WRITABLE":
        return "49modman cannot write to this game folder right now.";
      default:
        return result.message;
    }
  }

  $: validationCode = validation?.code;
  $: showDepotWorkflow =
    validationCode === "V49_SIGNATURE_MISMATCH" || validationCode === "V49_SIGNATURE_UNCONFIGURED";
  $: guidance = guidanceForCode(validationCode);
  $: validationSummary = summaryForValidation(validation);
  $: failedChecks = validation?.checks.filter((check) => !check.ok) ?? [];
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

  <section class="panel compact-panel onboarding-panel">
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
          <h3>{validation.ok ? "Install check passed" : "We couldn't verify this install yet"}</h3>
        </div>
        <p class="onboarding-friendly-copy">{validationSummary}</p>
        <p class="onboarding-friendly-copy">{guidance}</p>

        {#if validation.resolvedGamePath}
          <p class="onboarding-path-copy">Using folder: <strong>{validation.resolvedGamePath}</strong></p>
        {/if}

        {#if failedChecks.length > 0}
          <details class="onboarding-technical-details">
            <summary>Show technical details</summary>
            <div class="reference-table onboarding-technical-grid">
              {#each failedChecks as check}
                <div class="onboarding-check-row">
                  <div class="section-title-row">
                    <Icon label="Failed" name="warning" />
                    <strong>{check.message}</strong>
                  </div>
                  {#if check.detail}
                    <p class="empty-copy">{check.detail}</p>
                  {/if}
                  <p class="meta-label">Code: {check.code}</p>
                </div>
              {/each}
            </div>
          </details>
        {/if}
      </div>
    {/if}

    {#if showDepotWorkflow}
      <div class="onboarding-depot panel compact-panel">
        <div class="compact-heading compact-heading-left">
          <Icon label="Install steps" name="settings" />
          <h3>Installing Lethal Company v49</h3>
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
