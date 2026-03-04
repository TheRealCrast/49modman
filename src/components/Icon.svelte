<script lang="ts">
  import { iconMap, type IconName } from "../lib/icons";

  export let name: IconName;
  export let label = "";
  export let size = 18;
  export let forceWhite = false;
</script>

<img
  alt={label}
  class:icon-force-white={forceWhite}
  class="icon"
  height={size}
  src={iconMap[name]}
  width={size}
/>
