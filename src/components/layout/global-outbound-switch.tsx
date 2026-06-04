import { useLockFn } from 'ahooks'
import { Route, Waypoints } from 'lucide-react'

import { Button, ButtonGroup } from '@/components/tailwind'
import { useRuntimeConfig } from '@/hooks/data/use-clash'
import {
  useAppRefreshers,
  useClashConfigData,
} from '@/providers/app-data-context'
import { DEFAULT_CLASH_MODE, resolveClashMode } from '@/services/clash-mode'
import { patchClashMode } from '@/services/cmds'
import { queryClient } from '@/services/query-client'

export const GlobalOutboundSwitch = () => {
  const { clashConfig } = useClashConfigData()
  const { refreshClashConfig } = useAppRefreshers()
  const { data: runtimeConfig } = useRuntimeConfig()

  const mode = resolveClashMode(clashConfig?.mode, runtimeConfig?.mode)
  const intent = mode === 'direct' ? 'direct' : 'proxy'

  const setDirect = useLockFn(async () => {
    if (intent === 'direct') return

    queryClient.setQueryData(['getClashConfig'], (old: any) =>
      old ? { ...old, mode: 'direct' } : old,
    )
    queryClient.setQueryData(['getRuntimeConfig'], (old: any) =>
      old ? { ...old, mode: 'direct' } : old,
    )

    try {
      await patchClashMode('direct')
    } finally {
      await Promise.all([
        refreshClashConfig(),
        queryClient.invalidateQueries({ queryKey: ['getRuntimeConfig'] }),
      ])
    }
  })

  const setProxy = useLockFn(async () => {
    if (intent === 'proxy' && mode) return

    const proxyMode = mode && mode !== 'direct' ? mode : DEFAULT_CLASH_MODE

    queryClient.setQueryData(['getClashConfig'], (old: any) =>
      old ? { ...old, mode: proxyMode } : old,
    )
    queryClient.setQueryData(['getRuntimeConfig'], (old: any) =>
      old ? { ...old, mode: proxyMode } : old,
    )

    try {
      await patchClashMode('rule')
    } finally {
      await Promise.all([
        refreshClashConfig(),
        queryClient.invalidateQueries({ queryKey: ['getRuntimeConfig'] }),
      ])
    }
  })

  return (
    <ButtonGroup
      className="global-outbound-switch"
      size="small"
      aria-label="Global outbound mode"
    >
      <Button
        size="small"
        variant={intent === 'direct' ? 'primary' : 'outlined'}
        onClick={setDirect}
        startIcon={<Route className="h-3.5 w-3.5" />}
      >
        直连
      </Button>
      <Button
        size="small"
        variant={intent === 'proxy' ? 'primary' : 'outlined'}
        onClick={setProxy}
        startIcon={<Waypoints className="h-3.5 w-3.5" />}
      >
        代理链路
      </Button>
    </ButtonGroup>
  )
}
