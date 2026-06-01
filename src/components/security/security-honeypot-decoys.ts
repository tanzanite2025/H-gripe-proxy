export const DEFAULT_HONEYPOT_DECOY_ID = 'default-config-decoy'

export interface HoneypotDecoy {
  id: string
  name: string
  path: string
  kind: 'config-file'
  enabled: boolean
}

export function createDefaultHoneypotDecoys(): HoneypotDecoy[] {
  return [
    {
      id: DEFAULT_HONEYPOT_DECOY_ID,
      name: '默认配置诱饵',
      path: 'config_decoy.yaml',
      kind: 'config-file',
      enabled: true,
    },
  ]
}

export function getActiveHoneypotDecoyPath(
  decoys: HoneypotDecoy[],
  activeDecoyId: string,
): string {
  return decoys.find((decoy) => decoy.id === activeDecoyId)?.path ?? ''
}

export function updateActiveHoneypotDecoyPath(
  decoys: HoneypotDecoy[],
  activeDecoyId: string,
  path: string,
): HoneypotDecoy[] {
  return decoys.map((decoy) =>
    decoy.id === activeDecoyId ? { ...decoy, path } : decoy,
  )
}
