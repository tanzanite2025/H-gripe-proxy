import { healthcheckRuntimeProxyProvider } from '@/services/proxy-runtime'

export async function runProviderHealthChecks(providerNames: string[]) {
  if (!providerNames.length) {
    return
  }

  await Promise.allSettled(
    providerNames.map((provider) => healthcheckRuntimeProxyProvider(provider)),
  )
}
