import { invoke } from '@tauri-apps/api/core'

export async function getProfiles() {
  return invoke<IProfilesConfig>('get_profiles')
}

export async function enhanceProfiles() {
  return (
    (await invoke<ValidationOutcome>('enhance_profiles')).status === 'valid'
  )
}

export async function patchProfilesConfig(profiles: IProfilesConfig) {
  return (
    (await invoke<ValidationOutcome>('patch_profiles_config', { profiles }))
      .status === 'valid'
  )
}

export async function createProfile(
  item: Partial<IProfileItem>,
  fileData?: string | null,
) {
  return invoke<void>('create_profile', { item, fileData })
}

export async function createProfileFromLocalPath(
  item: Partial<IProfileItem>,
  path: string,
) {
  return invoke<void>('create_profile_from_local_path', { item, path })
}

export async function viewProfile(index: string) {
  return invoke<void>('view_profile', { index })
}

export async function readProfileFile(index: string) {
  return invoke<string>('read_profile_file', { index })
}

export async function saveProfileFile(index: string, fileData: string) {
  return (
    (
      await invoke<ValidationOutcome>('save_profile_file', {
        index,
        fileData,
      })
    ).status === 'valid'
  )
}

export async function importProfile(url: string, option?: IProfileOption) {
  return invoke<void>('import_profile', {
    url,
    option: option || { with_proxy: true },
  })
}

export async function reorderProfile(activeId: string, overId: string) {
  return invoke<void>('reorder_profile', {
    activeId,
    overId,
  })
}

export async function updateProfile(index: string, option?: IProfileOption) {
  return invoke<void>('update_profile', { index, option })
}

export async function deleteProfile(index: string) {
  return invoke<void>('delete_profile', { index })
}

export async function patchProfile(
  index: string,
  profile: Partial<IProfileItem>,
) {
  return invoke<void>('patch_profile', { index, profile })
}

export async function getNextUpdateTime(uid: string) {
  return invoke<number | null>('get_next_update_time', { uid })
}

export async function scriptValidateNotice(status: string, msg: string) {
  return invoke<void>('script_validate_notice', { status, msg })
}

export async function validateScriptFile(filePath: string) {
  return invoke<ValidationOutcome>('validate_script_file', { filePath })
}
