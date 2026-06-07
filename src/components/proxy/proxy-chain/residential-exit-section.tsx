import { Building2 } from 'lucide-react'

import type { ResidentialProxy, ResidentialProxyPool } from '@/services/coordinator'

import type { ProxyChainItem } from '../proxy-chain-types'

interface ResidentialExitSectionProps {
  proxyChain: ProxyChainItem[]
  residentialPool?: ResidentialProxyPool
  enabledResidentialProxies: ResidentialProxy[]
  onAddResidentialExit: (proxy: ResidentialProxy) => void
}

export const ResidentialExitSection = ({
  proxyChain,
  residentialPool,
  enabledResidentialProxies,
  onAddResidentialExit,
}: ResidentialExitSectionProps) => {
  return (
    <div className="mt-2 px-2">
      <div className="mb-2 flex items-center gap-1">
        <Building2 className="h-3.5 w-3.5 text-secondary" />
        <span className="text-xs font-medium text-text-secondary">住宅代理出口</span>
      </div>

      {enabledResidentialProxies.length > 0 ? (
        <div className="flex flex-wrap gap-1.5">
          {enabledResidentialProxies.map((proxy) => {
            const residentialName = `VERGE-RES-${proxy.name}`
            const isSelected = proxyChain.some(
              (item) => item.name === residentialName,
            )

            return (
              <button
                key={proxy.name}
                onClick={() => onAddResidentialExit(proxy)}
                disabled={isSelected}
                className={`rounded border px-2 py-1 text-xs transition-colors ${
                  isSelected
                    ? 'cursor-default border-orange-500/50 bg-orange-500/10 text-orange-400'
                    : 'cursor-pointer border-divider bg-card hover:border-primary hover:bg-primary/10 hover:text-primary'
                }`}
              >
                {proxy.name}
                {isSelected && ' 已加入'}
              </button>
            )
          })}
        </div>
      ) : (
        <div className="py-1 text-xs text-text-secondary/60">
          {residentialPool?.enabled
            ? '暂时没有已启用的住宅代理节点'
            : '住宅代理池尚未启用，启用后可以把住宅出口追加到代理链末端'}
        </div>
      )}
    </div>
  )
}
