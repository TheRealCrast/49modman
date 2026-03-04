<script lang="ts">
  import { iconMap, type IconName } from "../lib/icons";

  export let name: IconName;
  export let label = "";
  export let size = 18;
</script>

<img
  alt={label}
  class="icon"
  height={size}
  src={iconMap[name]}
  width={size}
/>
