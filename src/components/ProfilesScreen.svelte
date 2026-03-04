<script lang="ts">
  import type { Profile } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let profiles: Profile[] = [];
  export let selectedProfile: Profile | undefined;
  export let onSelectProfile: (profileId: string) => void;
</script>

<section class="screen-stack profile-screen">
  <div class="panel simple-hero compact-hero">
    <div>
      <h2>Switch active profile</h2>
      <p>Choose the active modpack.</p>
    </div>
    <div class="hero-inline">
      <span class="inline-label">Active</span>
      <strong>{selectedProfile?.name ?? "None selected"}</strong>
    </div>
  </div>

  <div class="panel list-panel">
    <div class="compact-heading compact-heading-left">
      <Icon label="Profiles" name="profiles" />
      <h2>Profiles</h2>
    </div>

    <div class="profile-list list-scroll">
      {#each profiles as profile}
        <button
          class:active={profile.id === selectedProfile?.id}
          class="profile-card"
          type="button"
          on:click={() => onSelectProfile(profile.id)}
        >
          <div class="profile-card-header">
            <div>
              <strong>{profile.name}</strong>
              <p>{profile.notes}</p>
            </div>
            <span class="profile-mode">{profile.id === selectedProfile?.id ? "Active" : profile.launchModeDefault}</span>
          </div>
          <div class="profile-card-footer">
            <span>{profile.installedMods.length} mods</span>
            <span>Played {profile.lastPlayed}</span>
          </div>
        </button>
      {/each}
    </div>
  </div>
</section>
