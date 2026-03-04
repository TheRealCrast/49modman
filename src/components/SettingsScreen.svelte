<script lang="ts">
  import type { ReferenceRow } from "../lib/types";
  import Icon from "./Icon.svelte";
  import StatusPill from "./StatusPill.svelte";

  export let rows: ReferenceRow[] = [];
  export let referenceSearchDraft = "";
  export let warningPrefs: {
    red: boolean;
    broken: boolean;
  };
  export let onReferenceSearchDraftChange: (value: string) => void;
  export let onSubmitReferenceSearch: () => void;
  export let onWarningPrefChange: (kind: "red" | "broken", enabled: boolean) => void;
  export let onSetReference: (packageId: string, versionId: string, state: "verified" | "broken" | "neutral") => void;
</script>

<section class="settings-grid">
  <section class="panel compact-panel">
    <div class="compact-heading compact-heading-left">
      <Icon label="Settings" name="settings" />
      <h2>Global settings</h2>
    </div>

    <div class="preference-list">
      <div class="switch-row">
        <div>
          <strong>Warn on red downloads</strong>
          <p>Ask before installing versions from the red zone.</p>
        </div>
        <button
          aria-pressed={warningPrefs.red}
          class="ghost-button icon-button toggle-icon-button"
          type="button"
          on:click={() => onWarningPrefChange("red", !warningPrefs.red)}
        >
          <Icon label={warningPrefs.red ? "Enabled" : "Disabled"} name={warningPrefs.red ? "check" : "circle"} />
          <span>{warningPrefs.red ? "On" : "Off"}</span>
        </button>
      </div>

      <div class="switch-row">
        <div>
          <strong>Warn on broken downloads</strong>
          <p>Ask before installing versions marked broken locally.</p>
        </div>
        <button
          aria-pressed={warningPrefs.broken}
          class="ghost-button icon-button toggle-icon-button"
          type="button"
          on:click={() => onWarningPrefChange("broken", !warningPrefs.broken)}
        >
          <Icon label={warningPrefs.broken ? "Enabled" : "Disabled"} name={warningPrefs.broken ? "check" : "circle"} />
          <span>{warningPrefs.broken ? "On" : "Off"}</span>
        </button>
      </div>
    </div>
  </section>

  <section class="panel list-panel">
    <div class="compact-heading compact-heading-left">
      <Icon label="Edit references" name="edit" />
      <h2>Reference library</h2>
    </div>

    <form class="toolbar-row" on:submit|preventDefault={onSubmitReferenceSearch}>
      <label class="search-field search-inline">
        <Icon label="Search references" name="search" />
        <input
          placeholder="Search by mod, version, note, or status"
          type="search"
          value={referenceSearchDraft}
          on:input={(event) => onReferenceSearchDraftChange((event.currentTarget as HTMLInputElement).value)}
        />
      </label>
      <button class="ghost-button icon-button" type="submit">
        <Icon label="Search" name="search" />
        <span>Search</span>
      </button>
    </form>

    <div class="reference-table list-scroll">
      {#each rows as row}
        <article class="reference-row">
          <div class="reference-main">
            <strong>{row.packageName}</strong>
            <p>
              {row.versionNumber} · published {row.publishedAt} · base {row.baseZone}
            </p>
            {#if row.note}
              <p class="reference-note">{row.note}</p>
            {/if}
          </div>

          <div class="reference-state">
            <StatusPill status={row.effectiveStatus} />
          </div>

          <div class="reference-actions">
            <button class="ghost-button icon-button" type="button" on:click={() => onSetReference(row.packageId, row.versionId, "verified")}>
              <Icon label="Mark verified" name="verified" />
              <span>Verified</span>
            </button>
            <button class="ghost-button danger-outline icon-button" type="button" on:click={() => onSetReference(row.packageId, row.versionId, "broken")}>
              <Icon label="Mark broken" name="broken" />
              <span>Broken</span>
            </button>
            <button class="ghost-button icon-button" type="button" on:click={() => onSetReference(row.packageId, row.versionId, "neutral")}>
              <Icon label="Clear override" name="x-close" />
              <span>Clear</span>
            </button>
          </div>
        </article>
      {/each}
    </div>
  </section>
</section>
