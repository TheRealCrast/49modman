import { invokeCommand } from "./client";
import type {
  CreateProfileInput,
  DeleteProfileResult,
  ProfileDetailDto,
  ProfilesStorageSummaryDto,
  ProfileSummaryDto,
  UpdateProfileInput
} from "../types";

export function listProfiles(): Promise<ProfileSummaryDto[]> {
  return invokeCommand("list_profiles");
}

export function getActiveProfile(): Promise<ProfileDetailDto | null> {
  return invokeCommand("get_active_profile");
}

export function setActiveProfile(profileId: string): Promise<ProfileDetailDto | null> {
  return invokeCommand("set_active_profile", { profileId });
}

export function createProfile(input: CreateProfileInput): Promise<ProfileDetailDto> {
  return invokeCommand("create_profile", { input });
}

export function updateProfile(input: UpdateProfileInput): Promise<ProfileDetailDto> {
  return invokeCommand("update_profile", { input });
}

export function deleteProfile(profileId: string): Promise<DeleteProfileResult> {
  return invokeCommand("delete_profile", { profileId });
}

export function getProfileDetail(profileId: string): Promise<ProfileDetailDto | null> {
  return invokeCommand("get_profile_detail", { profileId });
}

export function resetAllData(): Promise<void> {
  return invokeCommand("reset_all_data");
}

export function openProfilesFolder(): Promise<void> {
  return invokeCommand("open_profiles_folder");
}

export function openActiveProfileFolder(): Promise<void> {
  return invokeCommand("open_active_profile_folder");
}

export function getProfilesStorageSummary(): Promise<ProfilesStorageSummaryDto> {
  return invokeCommand("get_profiles_storage_summary");
}
