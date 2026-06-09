import { healthcheckProxyProvider } from 'tauri-plugin-mihomo-api'

export async function runProviderHealthChecks(providerNames: string[]) {
  if (!providerNames.length) {
    return
  }

  await Promise.allSettled(
    providerNames.map((provider) => healthcheckProxyProvider(provider)),
  )
}
