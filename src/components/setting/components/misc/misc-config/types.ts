export interface MiscConfigValues {
  autoCheckUpdate: boolean
  defaultLatencyTest: string
  defaultLatencyTimeout: number
}

export const createMiscConfigValues = (
  verge?: IVergeConfig | null,
): MiscConfigValues => ({
  autoCheckUpdate: verge?.auto_check_update ?? true,
  defaultLatencyTest: verge?.default_latency_test || '',
  defaultLatencyTimeout: verge?.default_latency_timeout || 10000,
})
