import { useEffect } from 'react'

interface UseProxyRefreshRecoveryOptions {
  onProxies: () => void
  proxiesData: any
}

export function useProxyRefreshRecovery({
  onProxies,
  proxiesData,
}: UseProxyRefreshRecoveryOptions) {
  useEffect(() => {
    if (!proxiesData) return

    const groups = proxiesData.groups || []
    const proxies = proxiesData.proxies || []

    if (groups.length === 0 || proxies.length === 0) {
      const handle = setTimeout(() => onProxies(), 500)
      return () => clearTimeout(handle)
    }
  }, [onProxies, proxiesData])
}
