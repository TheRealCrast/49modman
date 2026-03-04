<script lang="ts">
  import type { ResetProgressState, ResetProgressStep } from "../lib/types";

  export let state: ResetProgressState;

  const orderedSteps: ResetProgressStep[] = ["deleting", "restoring", "browse", "finalizing"];

  const stepCopy: Record<ResetProgressStep, string> = {
    deleting: "Delete local app data",
    restoring: "Restore default profile and settings",
    browse: "Refresh Browse catalog data",
    finalizing: "Finalize reset"
  };

  function stepIndex(step: ResetProgressStep) {
    return orderedSteps.indexOf(step);
  }

  function isDone(step: ResetProgressStep) {
    return stepIndex(step) < stepIndex(state.step);
  }
</script>

<div class="modal-scrim" role="presentation">
  <section aria-modal="true" class="modal-card reset-progress-card" role="dialog">
    <div class="loading-spinner" aria-hidden="true"></div>
    <h2>{state.title}</h2>
    <p>{state.message}</p>

    <div class="loading-steps" aria-label="Reset progress">
      {#each orderedSteps as step}
        <div class:active={state.step === step} class:done={isDone(step)} class="loading-step">
          <span class="loading-step-dot"></span>
          <span>{stepCopy[step]}</span>
        </div>
      {/each}
    </div>
  </section>
</div>
