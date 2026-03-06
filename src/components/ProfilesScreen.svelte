<script lang="ts">
  import type { CreateProfileInput, ProfileDetailDto, ProfileSummaryDto } from "../lib/types";
  import Icon from "./Icon.svelte";

  export let profiles: ProfileSummaryDto[] = [];
  export let selectedProfile: ProfileDetailDto | undefined;
  export let profileError: string | null = null;
  export let onSelectProfile: (profileId: string) => void | Promise<void>;
  export let onCreateProfile: (input: CreateProfileInput) => void | Promise<void>;
  export let onDeleteSelectedProfile: () => void | Promise<void>;
  export let onExportSelectedProfilePack: () => void | Promise<void>;
  export let onImportProfilePack: () => void | Promise<void>;
  export let onUpdateProfile: (input: {
    profileId: string;
    name: string;
    notes?: string;
    gamePath?: string;
    launchModeDefault?: "steam" | "direct";
  }) => void | Promise<void>;

  let isCreating = false;
  let isEditing = false;
  let createError: string | null = null;
  let name = "";
  let notes = "";
  let gamePath = "";
  let launchModeDefault: "steam" | "direct" = "steam";

  function formatBytes(value = 0) {
    if (value < 1024) {
      return `${value} B`;
    }

    if (value < 1024 * 1024) {
      return `${(value / 1024).toFixed(0)} KB`;
    }

    return `${(value / (1024 * 1024)).toFixed(1)} MB`;
  }

  function resetForm() {
    name = "";
    notes = "";
    gamePath = "";
    launchModeDefault = "steam";
    createError = null;
  }

  function populateEditForm() {
    name = selectedProfile?.name ?? "";
    notes = selectedProfile?.notes ?? "";
    gamePath = selectedProfile?.gamePath ?? "";
    launchModeDefault = selectedProfile?.launchModeDefault ?? "steam";
  }

  async function submitCreateProfile() {
    const trimmedName = name.trim();

    if (!trimmedName) {
      createError = "Profile name cannot be empty.";
      return;
    }

    if (profiles.some((profile) => profile.name.toLowerCase() === trimmedName.toLowerCase())) {
      createError = "A profile with that name already exists.";
      return;
    }

    createError = null;

    await onCreateProfile({
      name: trimmedName,
      notes,
      gamePath,
      launchModeDefault
    });

    resetForm();
    isCreating = false;
  }

  async function submitUpdateProfile() {
    if (!selectedProfile) {
      return;
    }

    await onUpdateProfile({
      profileId: selectedProfile.id,
      name,
      notes,
      gamePath,
      launchModeDefault
    });

    isEditing = false;
  }

  async function confirmDelete() {
    if (!selectedProfile || selectedProfile.isBuiltinDefault) {
      return;
    }

    if (!window.confirm(`Delete profile "${selectedProfile.name}"?`)) {
      return;
    }

    await onDeleteSelectedProfile();
  }
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
      <div class="section-title-row">
        <Icon label="Profiles" name="profiles" />
        <h2>Profiles</h2>
      </div>
      <div class="section-actions">
        <button
          class="ghost-button"
          type="button"
          on:click={() => {
            isEditing = false;
            if (!isCreating) {
              resetForm();
              isCreating = true;
            } else {
              resetForm();
              isCreating = false;
            }
          }}
        >
          {isCreating ? "Cancel" : "New profile"}
        </button>
        <button
          class="ghost-button"
          disabled={!selectedProfile}
          type="button"
          on:click={() => {
            isCreating = false;
            populateEditForm();
            isEditing = !isEditing;
            createError = null;
          }}
        >
          {isEditing ? "Cancel edit" : "Edit profile"}
        </button>
        <button
          class="ghost-button danger-outline"
          disabled={!selectedProfile || selectedProfile.isBuiltinDefault}
          type="button"
          on:click={confirmDelete}
        >
          Delete
        </button>
        <button
          class="ghost-button"
          disabled={!selectedProfile}
          type="button"
          on:click={() => void onExportSelectedProfilePack()}
        >
          Export .49pack
        </button>
        <button class="ghost-button" type="button" on:click={() => void onImportProfilePack()}>
          Import .49pack
        </button>
      </div>
    </div>

    {#if isCreating}
      <form class="profile-form-card" on:submit|preventDefault={submitCreateProfile}>
        <div class="profile-form-grid">
          <label class="form-field">
            <span>Name</span>
            <input bind:value={name} maxlength="80" required type="text" />
          </label>

          <label class="form-field">
            <span>Default launch mode</span>
            <select bind:value={launchModeDefault}>
              <option value="steam">steam</option>
              <option value="direct">direct</option>
            </select>
          </label>

          <label class="form-field form-field-wide">
            <span>Game path</span>
            <input bind:value={gamePath} type="text" />
          </label>

          <label class="form-field form-field-wide">
            <span>Notes</span>
            <textarea bind:value={notes} rows="3"></textarea>
          </label>
        </div>

        <div class="form-actions form-actions-end">
          <button class="solid-button" type="submit">Create</button>
          <button
            class="ghost-button"
            type="button"
            on:click={() => {
              resetForm();
              isCreating = false;
            }}
          >
            Cancel
          </button>
        </div>

        {#if createError}
          <p class="warning-copy danger form-error">{createError}</p>
        {/if}
      </form>
    {/if}

    {#if isEditing && selectedProfile}
      <form class="profile-form-card" on:submit|preventDefault={submitUpdateProfile}>
        <div class="profile-form-grid">
          <label class="form-field">
            <span>Name</span>
            <input
              bind:value={name}
              disabled={selectedProfile.isBuiltinDefault}
              maxlength="80"
              required
              type="text"
            />
          </label>

          <label class="form-field">
            <span>Default launch mode</span>
            <select bind:value={launchModeDefault}>
              <option value="steam">steam</option>
              <option value="direct">direct</option>
            </select>
          </label>

          <label class="form-field form-field-wide">
            <span>Game path</span>
            <input bind:value={gamePath} type="text" />
          </label>

          <label class="form-field form-field-wide">
            <span>Notes</span>
            <textarea bind:value={notes} rows="3"></textarea>
          </label>
        </div>

        <div class="form-actions form-actions-end">
          <button class="solid-button" type="submit">Save</button>
          <button class="ghost-button" type="button" on:click={() => (isEditing = false)}>
            Cancel
          </button>
        </div>
      </form>
    {/if}

    {#if profileError}
      <p class="warning-copy danger">{profileError}</p>
    {/if}

    <div class="profile-list list-scroll">
      {#each profiles as profile}
        <button
          class:active={profile.id === selectedProfile?.id}
          class="profile-card"
          type="button"
          on:click={() => void onSelectProfile(profile.id)}
        >
          <div class="profile-card-header">
            <div>
              <strong>{profile.name}</strong>
              <p>{profile.notes || "No notes yet."}</p>
            </div>
            <span class="profile-mode">
              {profile.id === selectedProfile?.id ? "Active" : profile.launchModeDefault}
            </span>
          </div>
          <div class="profile-card-footer">
            <span>{profile.gamePath || "No game path set"}</span>
            <span>
              {profile.isBuiltinDefault ? "Built-in default" : "Custom profile"} · {formatBytes(profile.profileSizeBytes)}
            </span>
          </div>
        </button>
      {/each}
    </div>
  </div>
</section>
