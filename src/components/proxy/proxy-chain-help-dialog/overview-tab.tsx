import { Alert } from '@/components/tailwind/Alert'

import { OVERVIEW_BENEFITS, OVERVIEW_TRADEOFFS } from './data'
import { HelpList } from './shared'

export const ProxyChainHelpOverviewTab = () => {
  return (
    <>
      <h6 className="mb-2 text-base font-semibold">什么是代理链？</h6>
      <p className="mb-4 text-sm">
        代理链会把多个代理节点串起来，流量会依次经过入口节点、中间节点和出口节点，最后再访问目标站点。
      </p>

      <Alert severity="info" className="mb-4">
        <p className="text-sm">
          <strong>工作路径：</strong>
          <br />
          {'设备 -> 入口节点 -> 中间节点 -> 出口节点 -> 目标网站'}
        </p>
      </Alert>

      <h6 className="mb-2 mt-4 text-sm font-semibold">主要优势</h6>
      <HelpList items={OVERVIEW_BENEFITS} />

      <h6 className="mb-2 mt-4 text-sm font-semibold">主要代价</h6>
      <HelpList items={OVERVIEW_TRADEOFFS} />
    </>
  )
}
