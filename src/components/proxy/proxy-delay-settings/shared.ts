import {
  DEFAULT_DELAY_TIMEOUT,
  resolveVergeDelayTimeout,
} from '@/services/delay-config'

export interface DelaySettingsFormState {
  defaultLatencyTest: string
  defaultLatencyTimeout: number
}

export const parsePositiveInt = (
  value: string,
  fallback = DEFAULT_DELAY_TIMEOUT,
) => {
  const parsed = parseInt(value, 10)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback
}

export const createDelaySettingsState = (
  verge?: IVergeConfig | null,
): DelaySettingsFormState => ({
  defaultLatencyTest: verge?.default_latency_test || '',
  defaultLatencyTimeout: resolveVergeDelayTimeout(verge),
})

export const areDelaySettingsEqual = (
  left: DelaySettingsFormState,
  right: DelaySettingsFormState,
) =>
  left.defaultLatencyTest === right.defaultLatencyTest &&
  left.defaultLatencyTimeout === right.defaultLatencyTimeout
