import { useEffect } from 'react'

import delayManager from '@/services/delay'

export function useGroupDelayRefresh(
  groupName: string,
  onRefresh: () => void,
) {
  useEffect(() => {
    let last = 0

    delayManager.setGroupListener(groupName, () => {
      const now = Date.now()
      if (now - last > 666) {
        last = now
        onRefresh()
      }
    })

    return () => {
      delayManager.removeGroupListener(groupName)
    }
  }, [groupName, onRefresh])
}
