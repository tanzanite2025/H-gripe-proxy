const CORE_LOG_LEVEL_OPTIONS = [
  'debug',
  'info',
  'warning',
  'error',
  'silent',
] as const

const CORE_LOG_LEVEL_SET = new Set<LogLevel>(CORE_LOG_LEVEL_OPTIONS)

export { CORE_LOG_LEVEL_OPTIONS }

export const normalizeCoreLogLevel = (
  value?: string | null,
): LogLevel => {
  if (value === 'warn') {
    return 'warning'
  }

  if (value && CORE_LOG_LEVEL_SET.has(value as LogLevel)) {
    return value as LogLevel
  }

  return 'info'
}
