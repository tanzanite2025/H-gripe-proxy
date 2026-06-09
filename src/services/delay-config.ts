export const DEFAULT_DELAY_TEST_URL = 'https://cp.cloudflare.com/generate_204'
export const DEFAULT_DELAY_TIMEOUT = 10000

export function normalizeDelayTestUrl(url?: string | null) {
  const trimmed = url?.trim()
  if (!trimmed) return DEFAULT_DELAY_TEST_URL

  if (trimmed.startsWith('http://') && trimmed.includes('/generate_204')) {
    return `https://${trimmed.slice('http://'.length)}`
  }

  return trimmed
}

export function resolveDelayTimeout(timeout?: number | null) {
  return typeof timeout === 'number' && Number.isFinite(timeout) && timeout > 0
    ? timeout
    : DEFAULT_DELAY_TIMEOUT
}

export function resolveVergeDelayTestUrl(verge?: IVergeConfig | null) {
  return normalizeDelayTestUrl(verge?.default_latency_test)
}

export function resolveVergeDelayTimeout(verge?: IVergeConfig | null) {
  return resolveDelayTimeout(verge?.default_latency_timeout)
}
