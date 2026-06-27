export interface ClashPortViewerRef {
  open: () => void
  close: () => void
}

export interface ClashPortValues {
  mixedPort: number
  socksPort: number
  socksEnabled: boolean
  httpPort: number
  httpEnabled: boolean
}

export type ClashPortNumberKey =
  | 'mixedPort'
  | 'socksPort'
  | 'httpPort'

export interface ClashPortRowConfig {
  key: ClashPortNumberKey
  label: string
  enabledKey: keyof ClashPortValues | null
}
