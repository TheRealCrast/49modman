<script lang="ts">
  import type { IconName } from "../lib/icons";
  import Icon from "./Icon.svelte";

  export let title: string;
  export let description: string;
  export let confirmLabel = "Confirm";
  export let confirmIcon: IconName = "check";
  export let isDanger = false;
  export let onCancel: () => void;
  export let onConfirm: () => void;
</script>

<div class="modal-scrim" role="presentation">
  <section aria-modal="true" class="modal-card simple-confirm-modal" role="dialog">
    <div class="compact-heading">
      <Icon label="Warning" name="warning" />
      <h2>{title}</h2>
    </div>
    <p class="modal-copy">{description}</p>

    <div class="modal-actions">
      <button class="ghost-button icon-button" type="button" on:click={onCancel}>
        <Icon label="Cancel" name="x-close" />
        <span>Cancel</span>
      </button>
      <button
        class={`${isDanger ? "danger-button" : "solid-button"} icon-button`}
        type="button"
        on:click={onConfirm}
      >
        <Icon label={confirmLabel} name={confirmIcon} forceWhite={true} />
        <span>{confirmLabel}</span>
      </button>
    </div>
  </section>
</div>
