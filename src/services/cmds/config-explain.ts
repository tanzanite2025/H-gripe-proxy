import { invoke } from '@tauri-apps/api/core'

export type ConfigDiffChangeType = 'added' | 'removed' | 'modified'

export interface ConfigDiffChange {
  path: string
  changeType: ConfigDiffChangeType
  beforeType: string | null
  afterType: string | null
}

export interface ConfigSectionSummary {
  path: string
  beforeCount: number | null
  afterCount: number | null
  delta: number | null
}

export interface ConfigDiffReport {
  changed: boolean
  explanation: string
  changes: ConfigDiffChange[]
  sectionSummaries: ConfigSectionSummary[]
}

export async function explainConfigDiff(
  beforeYaml: string,
  afterYaml: string,
) {
  return invoke<ConfigDiffReport>('explain_config_diff', {
    beforeYaml,
    afterYaml,
  })
}
