import { updateProxyChainConfigInRuntime } from '@/services/cmds'

import type { ProxyChainRuntimeIntent } from './proxy-chain-types'

export const clearProxyChainRuntimeConfig = () =>
  updateProxyChainConfigInRuntime(null)

export const applyProxyChainRuntimeIntent = (
  intent: ProxyChainRuntimeIntent,
) => updateProxyChainConfigInRuntime(intent.runtimePayload)
