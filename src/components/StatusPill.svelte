<script lang="ts">
  import type { EffectiveStatus } from "../lib/types";

  export let status: EffectiveStatus;
  export let compact = false;

  const labels: Record<EffectiveStatus, string> = {
    broken: "Broken",
    verified: "Verified",
    green: "Compatible",
    yellow: "Watchlist",
    orange: "Outdated",
    red: "Incompatible"
  };
</script>

<span class:compact class={`status-pill ${status}`}>
  {labels[status]}
</span>
