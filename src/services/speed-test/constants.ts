export const SPEED_TEST_ENDPOINTS = {
  download: 'https://speed.cloudflare.com/__down',
  upload: 'https://speed.cloudflare.com/__up',
} as const

export const SPEED_TEST_SETTINGS = {
  downloadBytes: 10 * 1024 * 1024,
  uploadBytes: 5 * 1024 * 1024,
  downloadSampleIntervalMs: 200,
  downloadConnectTimeoutMs: 30_000,
  uploadConnectTimeoutMs: 30_000,
  latencyAttempts: 10,
  latencyDelayMs: 100,
  latencyConnectTimeoutMs: 5_000,
  packetLossAttempts: 20,
  packetLossDelayMs: 50,
  packetLossConnectTimeoutMs: 3_000,
} as const

export const buildDownloadEndpoint = (bytes: number) =>
  `${SPEED_TEST_ENDPOINTS.download}?bytes=${bytes}`
