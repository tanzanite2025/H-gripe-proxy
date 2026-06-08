export interface MiscConfigValues {
  appLogLevel: string
  appLogMaxSize: number
  appLogMaxCount: number
  autoCloseConnection: boolean
  autoCheckUpdate: boolean
  enableBuiltinEnhanced: boolean
  proxyLayoutColumn: number
  enableAutoDelayDetection: boolean
  autoDelayDetectionIntervalMinutes: number
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
  autoCloseConnection: verge?.auto_close_connection ?? true,
  autoCheckUpdate: verge?.auto_check_update ?? true,
  enableBuiltinEnhanced: verge?.enable_builtin_enhanced ?? true,
  proxyLayoutColumn: verge?.proxy_layout_column || 6,
  enableAutoDelayDetection: verge?.enable_auto_delay_detection ?? false,
  autoDelayDetectionIntervalMinutes:
    verge?.auto_delay_detection_interval_minutes ?? 5,
  defaultLatencyTest: verge?.default_latency_test || '',
  autoLogClean: verge?.auto_log_clean || 0,
  defaultLatencyTimeout: verge?.default_latency_timeout || 10000,
})
