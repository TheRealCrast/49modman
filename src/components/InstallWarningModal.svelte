<script lang="ts">
  import DoNotShowAgainToggle from "./DoNotShowAgainToggle.svelte";
  import Icon from "./Icon.svelte";

  export let title: string;
  export let description: string;
  export let note: string | undefined;
  export let onCancel: () => void;
  export let onConfirm: (doNotShowAgain: boolean) => void;

  let doNotShowAgain = false;
</script>

<div class="modal-scrim" role="presentation">
  <section aria-modal="true" class="modal-card" role="dialog">
    <div class="compact-heading">
      <Icon label="Warning" name="warning" />
      <h2>{title}</h2>
    </div>
    <p class="modal-copy">{description}</p>

    {#if note}
      <div class="modal-note">
        <p>{note}</p>
      </div>
    {/if}

    <DoNotShowAgainToggle
      checked={doNotShowAgain}
      onToggle={() => (doNotShowAgain = !doNotShowAgain)}
    />

    <div class="modal-actions">
      <button class="ghost-button icon-button" type="button" on:click={onCancel}>
        <Icon label="Cancel" name="x-close" />
        <span>Cancel</span>
      </button>
      <button class="danger-button icon-button" type="button" on:click={() => onConfirm(doNotShowAgain)}>
        <Icon label="Download anyway" name="download" forceWhite={true} />
        <span>Download anyway</span>
      </button>
    </div>
  </section>
</div>
