import { invoke } from '@tauri-apps/api/core'

export async function createWebdavBackup() {
  return invoke<void>('create_webdav_backup')
}

export async function createLocalBackup() {
  return invoke<void>('create_local_backup')
}

export async function deleteWebdavBackup(filename: string) {
  return invoke<void>('delete_webdav_backup', { filename })
}

export async function deleteLocalBackup(filename: string) {
  return invoke<void>('delete_local_backup', { filename })
}

export async function restoreWebDavBackup(filename: string) {
  return invoke<void>('restore_webdav_backup', { filename })
}

export async function restoreLocalBackup(filename: string) {
  return invoke<void>('restore_local_backup', { filename })
}

export async function importLocalBackup(source: string) {
  return invoke<void>('import_local_backup', { source })
}

export async function exportLocalBackup(filename: string, destination: string) {
  return invoke<void>('export_local_backup', { filename, destination })
}

export async function saveWebdavConfig(
  url: string,
  username: string,
  password: string,
) {
  return invoke<void>('save_webdav_config', {
    url,
    username,
    password,
  })
}

export async function listWebDavBackup() {
  const list: IWebDavFile[] = await invoke<IWebDavFile[]>('list_webdav_backup')
  list.map((item) => {
    item.filename = item.href.split('/').pop() as string
  })
  return list
}

export async function listLocalBackup() {
  return invoke<ILocalBackupFile[]>('list_local_backup')
}
