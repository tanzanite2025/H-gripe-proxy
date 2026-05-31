import { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'

import { ProxyChain, type ProxyChainItem } from './proxy-chain'

interface ProxyChainDrawerProps {
  open: boolean
  proxyChain: ProxyChainItem[]
  onUpdateChain: (chain: ProxyChainItem[]) => void
  chainConfigData?: string | null
  mode?: string
  selectedGroup?: string | null
  onClose: () => void
}

export const ProxyChainDrawer = ({
  open,
  proxyChain,
  onUpdateChain,
  chainConfigData,
  mode,
  selectedGroup,
  onClose,
}: ProxyChainDrawerProps) => {
  const [visible, setVisible] = useState(false)
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    if (open) {
      setMounted(true)
      requestAnimationFrame(() => {
        requestAnimationFrame(() => setVisible(true))
      })
    } else {
      setVisible(false)
      const timer = setTimeout(() => setMounted(false), 300)
      return () => clearTimeout(timer)
    }
  }, [open])

  if (!mounted) return null

  return createPortal(
    <div
        className={`fixed inset-x-0 bottom-0 z-50 flex flex-col transition-transform duration-300 ease-out ${
          visible ? 'translate-y-0' : 'translate-y-full'
        }`}
        style={{ height: '35vh' }}
      >
        <div className="flex-1 bg-card border-t border-divider rounded-t-xl shadow-2xl overflow-hidden flex flex-col">
          {/* 顶部拖拽条 */}
          <div className="flex justify-center pt-2 pb-0 shrink-0">
            <div className="w-10 h-1 rounded-full bg-gray-500/40" />
          </div>
          {/* 内容区 */}
          <div className="flex-1 overflow-hidden">
            <ProxyChain
              proxyChain={proxyChain}
              onUpdateChain={onUpdateChain}
              chainConfigData={chainConfigData}
              mode={mode}
              selectedGroup={selectedGroup}
              onClose={onClose}
              bare
            />
          </div>
        </div>
      </div>,
    document.body,
  )
}
