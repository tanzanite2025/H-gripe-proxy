import { ScrollTopButton } from '../../../layout/scroll-top-button'
import {
  DEFAULT_HOVER_DELAY,
  ProxyGroupNavigator,
} from '../../proxy-group-navigator'
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
      {controller.isRuleMode && (
        <ProxyGroupNavigator
          proxyGroupNames={controller.proxyGroupNames}
          onGroupLocation={controller.handleGroupLocationByNameWithScroll}
          enableHoverJump={true}
          hoverDelay={DEFAULT_HOVER_DELAY}
        />
      )}

      <ProxyVirtualList {...controller.createProxyListProps('calc(100% - 14px)')} />
      <ScrollTopButton
        show={controller.showScrollTop}
        onClick={controller.scrollToTop}
      />
    </div>
  )
}
