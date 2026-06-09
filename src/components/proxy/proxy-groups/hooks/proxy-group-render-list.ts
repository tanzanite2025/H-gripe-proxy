import type { HeadState } from '../../use-head-state'
import type { IRenderItem } from '../../use-render-list'

type ScrollAlign = 'center' | 'start'

export interface ProxyGroupScrollTarget {
  align: ScrollAlign
  index: number
}

export const getGroupHeadStateFromRenderList = (
  renderList: IRenderItem[],
  groupName: string,
): HeadState | undefined => {
  const headItem = renderList.find(
    (item) => item.type === 1 && item.group?.name === groupName,
  )

  return headItem?.headState
}

export const getProxyGroupNamesFromRenderList = (renderList: IRenderItem[]) => {
  return renderList
    .filter((item) => item.type === 0 && item.group?.name)
    .map((item) => item.group!.name)
}

export const findProxyGroupHeaderIndex = (
  renderList: IRenderItem[],
  groupName: string,
) => {
  return renderList.findIndex(
    (item) => item.type === 0 && item.group?.name === groupName,
  )
}

export const findProxyGroupScrollTarget = (
  renderList: IRenderItem[],
  group: IProxyGroupItem,
): ProxyGroupScrollTarget | null => {
  const currentProxyName = group.now?.trim() || ''

  if (currentProxyName) {
    const proxyIndex = renderList.findIndex(
      (item) =>
        item.group?.name === group.name &&
        ((item.type === 2 && item.proxy?.name === currentProxyName) ||
          (item.type === 4 &&
            item.proxyCol?.some((proxy) => proxy.name === currentProxyName))),
    )

    if (proxyIndex >= 0) {
      return {
        align: 'center',
        index: proxyIndex,
      }
    }
  }

  const groupIndex = findProxyGroupHeaderIndex(renderList, group.name)
  if (groupIndex < 0) {
    return null
  }

  return {
    align: 'start',
    index: groupIndex,
  }
}
