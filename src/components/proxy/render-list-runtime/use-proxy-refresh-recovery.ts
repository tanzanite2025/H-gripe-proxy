import { useEffect } from 'react'

interface UseProxyRefreshRecoveryOptions {
  mode: string
  onProxies: () => void
  proxiesData: any
}

export function useProxyRefreshRecovery({
  mode,
  onProxies,
  proxiesData,
}: UseProxyRefreshRecoveryOptions) {
  useEffect(() => {
    if (!proxiesData) return

    const groups = proxiesData.groups || []
    const proxies = proxiesData.proxies || []

    if (
      (mode === 'rule' && groups.length === 0) ||
      (mode === 'global' && proxies.length < 2)
    ) {
      const handle = setTimeout(() => onProxies(), 500)
      return () => clearTimeout(handle)
    }
  }, [mode, onProxies, proxiesData])
}
