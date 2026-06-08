export interface MiscConfigValues {
  appLogLevel: string
  appLogMaxSize: number
  appLogMaxCount: number
  autoCheckUpdate: boolean
  enableBuiltinEnhanced: boolean
  proxyLayoutColumn: number
  defaultLatencyTest: string
  autoLogClean: number
  defaultLatencyTimeout: number
}

export const createMiscConfigValues = (
  verge?: IVergeConfig | null,
): MiscConfigValues => ({
  appLogLevel: verge?.app_log_level ?? 'warn',
  appLogMaxSize: verge?.app_log_max_size ?? 128,
  appLogMaxCount: verge?.app_log_max_count ?? 8,
  autoCheckUpdate: verge?.auto_check_update ?? true,
  enableBuiltinEnhanced: verge?.enable_builtin_enhanced ?? true,
  proxyLayoutColumn: verge?.proxy_layout_column || 6,
  defaultLatencyTest: verge?.default_latency_test || '',
  autoLogClean: verge?.auto_log_clean || 0,
  defaultLatencyTimeout: verge?.default_latency_timeout || 10000,
})
