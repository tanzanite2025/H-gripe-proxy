export interface MiscConfigValues {
  autoCheckUpdate: boolean
  proxyLayoutColumn: number
  defaultLatencyTest: string
  defaultLatencyTimeout: number
}

export const createMiscConfigValues = (
  verge?: IVergeConfig | null,
): MiscConfigValues => ({
  autoCheckUpdate: verge?.auto_check_update ?? true,
  proxyLayoutColumn: verge?.proxy_layout_column || 6,
  defaultLatencyTest: verge?.default_latency_test || '',
  defaultLatencyTimeout: verge?.default_latency_timeout || 10000,
})
