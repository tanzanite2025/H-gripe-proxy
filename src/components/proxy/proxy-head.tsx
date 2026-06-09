import { useEffect, useState } from 'react'

import { useVerge } from '@/hooks/system'
import delayManager from '@/services/delay'
import { resolveVergeDelayTestUrl } from '@/services/delay-config'

import { ProxyHeadActions } from './proxy-head/proxy-head-actions'
import { ProxyHeadInput } from './proxy-head/proxy-head-input'
import type { HeadState } from './use-head-state'

interface Props {
  className?: string
  url?: string
  groupName: string
  headState: HeadState
  onLocation: () => void
  onCheckDelay: () => void
  onHeadState: (val: Partial<HeadState>) => void
}

export const ProxyHead = ({
  className = '',
  url,
  groupName,
  headState,
  onHeadState,
  onLocation,
  onCheckDelay,
}: Props) => {
  const [autoFocus, setAutoFocus] = useState(false)

  useEffect(() => {
    const timer = setTimeout(() => setAutoFocus(true), 100)
    return () => clearTimeout(timer)
  }, [])

  const { verge } = useVerge()
  const defaultLatencyUrl = resolveVergeDelayTestUrl(verge)

  useEffect(() => {
    delayManager.setUrl(
      groupName,
      headState.testUrl?.trim() || url || defaultLatencyUrl,
    )
  }, [defaultLatencyUrl, groupName, headState.testUrl, url])

  return (
    <div className={`flex items-center gap-1 ${className}`}>
      <ProxyHeadActions
        groupName={groupName}
        headState={headState}
        onLocation={onLocation}
        onCheckDelay={onCheckDelay}
        onHeadState={onHeadState}
      />
      <ProxyHeadInput
        autoFocus={autoFocus}
        headState={headState}
        onHeadState={onHeadState}
      />
    </div>
  )
}
