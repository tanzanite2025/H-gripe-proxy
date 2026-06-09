import type { StrategyGroupLoadWarning } from './types'

export const resolveLoadWarningMessage = (
  warnings: StrategyGroupLoadWarning[],
): string => {
  if (warnings.includes('configNotReady')) {
    return '当前策略池覆写配置还没准备好，先展示全部节点；配置加载完成后即可保存。'
  }

  if (warnings.includes('groupsReadFailed')) {
    return '策略池现有配置暂时没读到，先展示全部节点；保存后会按当前勾选重建。'
  }

  return ''
}
