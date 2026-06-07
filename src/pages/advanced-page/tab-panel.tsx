import type { ReactNode } from 'react'

import { Box } from '@/components/tailwind'

import type { AdvancedTabId } from './constants'

interface TabPanelProps {
  activeTab: AdvancedTabId
  tabId: AdvancedTabId
  children: ReactNode
}

export function AdvancedTabPanel({
  activeTab,
  tabId,
  children,
}: TabPanelProps) {
  return (
    <div
      role="tabpanel"
      hidden={activeTab !== tabId}
      id={`advanced-tabpanel-${tabId}`}
      aria-labelledby={`advanced-tab-${tabId}`}
    >
      {activeTab === tabId ? <Box className="py-2">{children}</Box> : null}
    </div>
  )
}
