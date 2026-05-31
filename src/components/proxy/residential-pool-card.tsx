import { useQuery } from '@tanstack/react-query'
import { Building2 } from 'lucide-react'

import { EnhancedCard } from '@/components/home/enhanced-card'
import { getAdvancedConfig, type ResidentialProxyPool } from '@/services/coordinator'

const proxyTypeLabels: Record<string, string> = {
  socks5: 'SOCKS5',
  http: 'HTTP',
  ss: 'Shadowsocks',
  vmess: 'VMess',
  trojan: 'Trojan',
}

export function ResidentialPoolCard() {
  const { data: config } = useQuery({
    queryKey: ['advancedConfig'],
    queryFn: getAdvancedConfig,
    staleTime: 30_000,
  })

  const pool: ResidentialProxyPool | undefined = config?.residential_pool

  if (!pool) {
    return (
      <EnhancedCard
        title="住宅代理池"
        icon={<Building2 className="h-4 w-4" />}
        iconColor="secondary"
      >
        <div className="text-xs text-gray-500 dark:text-gray-400 text-center py-2">
          加载中...
        </div>
      </EnhancedCard>
    )
  }

  const activeProxies = pool.proxies.filter((p) => p.enabled)
  const totalProxies = pool.proxies.length

  return (
    <EnhancedCard
      title="住宅代理池"
      icon={<Building2 className="h-4 w-4" />}
      iconColor="secondary"
    >
      {!pool.enabled ? (
        <div className="text-xs text-gray-500 dark:text-gray-400 text-center py-2">
          住宅代理池未启用
        </div>
      ) : (
        <div className="space-y-2">
          {/* 概览 */}
          <div className="flex items-center justify-between text-xs">
            <span className="text-gray-500 dark:text-gray-400">代理节点</span>
            <span className="font-medium">
              {activeProxies.length} / {totalProxies} 启用
            </span>
          </div>

          {/* 节点列表 */}
          {activeProxies.length > 0 ? (
            <div className="space-y-1">
              {activeProxies.slice(0, 5).map((proxy) => (
                <div
                  key={proxy.name}
                  className="flex items-center justify-between text-xs px-2 py-1 rounded bg-gray-50 dark:bg-gray-800"
                >
                  <span className="font-medium truncate mr-2">{proxy.name}</span>
                  <span className="text-gray-500 dark:text-gray-400 shrink-0">
                    {proxyTypeLabels[proxy.proxyType] || proxy.proxyType}
                  </span>
                </div>
              ))}
              {activeProxies.length > 5 && (
                <div className="text-xs text-gray-500 dark:text-gray-400 text-center">
                  还有 {activeProxies.length - 5} 个节点...
                </div>
              )}
            </div>
          ) : (
            <div className="text-xs text-gray-500 dark:text-gray-400 text-center">
              暂无启用的住宅代理节点
            </div>
          )}
        </div>
      )}
    </EnhancedCard>
  )
}
