import { invoke } from '@tauri-apps/api/core'

export async function readChinaRulesFile() {
  return invoke<string>('read_china_rules_file')
}

export async function saveChinaRulesFile(fileData: string) {
  return (
    (
      await invoke<ValidationOutcome>('save_china_rules_file', {
        fileData,
      })
    ).status === 'valid'
  )
}
