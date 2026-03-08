<script lang="ts">
  import Icon from "./Icon.svelte";

  export let checked = false;
  export let disabled = false;
  export let onToggle: () => void;
</script>

<button
  aria-pressed={checked}
  class="ghost-button icon-button toggle-icon-button modal-toggle"
  type="button"
  {disabled}
  on:click={onToggle}
>
  <Icon label={checked ? "Enabled" : "Disabled"} name={checked ? "check" : "circle"} />
  <span>Do not show this again</span>
</button>
