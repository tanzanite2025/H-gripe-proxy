import { ScrollTopButton } from '../../../layout/scroll-top-button'
import type { ProxyGroupsController } from '../hooks/use-proxy-groups-controller'

import { ProxyVirtualList } from './proxy-virtual-list'

interface ProxyGroupsDefaultViewProps {
  controller: ProxyGroupsController
}

export function ProxyGroupsDefaultView({
  controller,
}: ProxyGroupsDefaultViewProps) {
  return (
    <div style={{ position: 'relative', height: '100%', willChange: 'transform' }}>
      <ProxyVirtualList {...controller.createProxyListProps('calc(100% - 14px)')} />
      <ScrollTopButton
        show={controller.showScrollTop}
        onClick={controller.scrollToTop}
      />
    </div>
  )
}
