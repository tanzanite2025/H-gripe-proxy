import {
  BEST_PRACTICE_PERFORMANCE,
  BEST_PRACTICE_SECURITY,
  BEST_PRACTICE_SELECTION,
} from './data'
import { HelpList } from './shared'

export const ProxyChainHelpBestPracticesTab = () => {
  return (
    <>
      <h6 className="mb-2 text-base font-semibold">最佳实践</h6>

      <h6 className="mb-2 mt-4 text-sm font-semibold">节点选择建议</h6>
      <HelpList items={BEST_PRACTICE_SELECTION} />

      <div className="my-4 border-t border-divider" />

      <h6 className="mb-2 text-sm font-semibold">性能优化建议</h6>
      <HelpList items={BEST_PRACTICE_PERFORMANCE} />

      <div className="my-4 border-t border-divider" />

      <h6 className="mb-2 text-sm font-semibold">安全建议</h6>
      <HelpList items={BEST_PRACTICE_SECURITY} />
    </>
  )
}
