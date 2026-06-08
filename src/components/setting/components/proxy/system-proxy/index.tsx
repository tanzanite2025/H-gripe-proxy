import { useLockFn } from 'ahooks'
import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
} from 'react'

import { DialogRef } from '@/components/base'
import { useSystemProxyState, useVerge } from '@/hooks/system'
import { useClashConfigData, useSystemData } from '@/providers/app-data-context'
import {
  getAutotemProxy,
  getSystemProxy,
  patchVergeConfig,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import getSystem from '@/utils/misc'

import { SystemProxyUI } from '../system-proxy-ui'
import {
  createSystemProxyFormValue,
  DEFAULT_PAC,
  FALLBACK_HOST_OPTIONS,
  sleep,
} from './constants'
import { loadSystemProxyHostOptions } from './host-options'
import { getDefaultBypass } from './helpers'
import { buildSystemProxyPatch } from './patch'
import type { SystemProxyFormValue } from './types'
import {
  createBypassValidator,
  hasInvalidBypassValue,
  isValidProxyHost,
} from './validation'

export const SysproxyViewer = forwardRef<DialogRef>((props, ref) => {
  const systemName = getSystem()
  const isWindows = systemName === 'windows'
  const bypassValidator = useMemo(
    () => createBypassValidator(isWindows),
    [isWindows],
  )

  const [open, setOpen] = useState(false)
  const [editorOpen, setEditorOpen] = useState(false)
  const [pacEditorValue, setPacEditorValue] = useState(DEFAULT_PAC)
  const [pacEditorSavedValue, setPacEditorSavedValue] = useState(DEFAULT_PAC)
  const [saving, setSaving] = useState(false)
  const [hostOptions, setHostOptions] = useState(FALLBACK_HOST_OPTIONS)

  const { verge, patchVerge, mutateVerge } = useVerge()
  const { clashConfig } = useClashConfigData()
  const { indicator: isProxyReallyEnabled, invalidateProxyState } =
    useSystemProxyState()
  const { systemProxyAddress } = useSystemData()

  const {
    enable_system_proxy: enabled,
    proxy_auto_config,
    pac_file_content,
    enable_proxy_guard,
    enable_bypass_check,
    use_default_bypass,
    system_proxy_bypass,
    proxy_guard_duration,
    proxy_host,
  } = verge ?? {}

  const [value, setValue] = useState<SystemProxyFormValue>(() =>
    createSystemProxyFormValue(verge),
  )

  const separator = useMemo(() => (isWindows ? ';' : ','), [isWindows])
  const prevMixedPortRef = useRef(clashConfig?.mixedPort)

  useEffect(() => {
    const mixedPort = clashConfig?.mixedPort
    if (!mixedPort || mixedPort === prevMixedPortRef.current) {
      return
    }

    prevMixedPortRef.current = mixedPort
    if (!enabled) {
      return
    }

    const updateProxy = async () => {
      try {
        const currentSysProxy = await getSystemProxy()
        const currentAutoProxy = await getAutotemProxy()

        if (value.pac ? currentAutoProxy?.enable : currentSysProxy?.enable) {
          await patchVergeConfig({ enable_system_proxy: false })
          await sleep(200)
          await patchVergeConfig({ enable_system_proxy: true })
          await invalidateProxyState()
        }
      } catch (error) {
        showNotice.error(error)
      }
    }

    void updateProxy()
  }, [clashConfig?.mixedPort, enabled, invalidateProxyState, value.pac])

  const systemProxyAddressText = useMemo(() => {
    if (!clashConfig) return '-'

    if (value.pac) {
      const host = value.proxy_host || FALLBACK_HOST_OPTIONS[0]
      const port = verge?.verge_mixed_port || clashConfig.mixedPort || 7897
      return `${host}:${port}`
    }

    return systemProxyAddress
  }, [
    clashConfig,
    systemProxyAddress,
    value.pac,
    value.proxy_host,
    verge?.verge_mixed_port,
  ])

  const currentPacUrl = useMemo(() => {
    const host = value.proxy_host || FALLBACK_HOST_OPTIONS[0]
    const port = import.meta.env.DEV ? 11233 : 33331
    return `http://${host}:${port}/commands/pac`
  }, [value.proxy_host])

  const bypassError = hasInvalidBypassValue(value, bypassValidator)

  const defaultBypass = () => getDefaultBypass(systemName, isWindows)

  const fetchNetworkInterfaces = async () => {
    const options = await loadSystemProxyHostOptions()
    setHostOptions(options)
  }

  const openPacEditor = () => {
    const nextPac = value.pac_content ?? DEFAULT_PAC
    setPacEditorValue(nextPac)
    setPacEditorSavedValue(nextPac)
    setEditorOpen(true)
  }

  const handleSavePac = useLockFn(async () => {
    const nextPac =
      pacEditorValue.trim().length > 0 ? pacEditorValue : DEFAULT_PAC

    setValue((current) => ({ ...current, pac_content: nextPac }))
    setPacEditorSavedValue(nextPac)
  })

  useImperativeHandle(ref, () => ({
    open: () => {
      setOpen(true)
      setValue(createSystemProxyFormValue(verge))
      void fetchNetworkInterfaces()
    },
    close: () => setOpen(false),
  }))

  const onSave = useLockFn(async () => {
    if (value.duration < 1) {
      showNotice.error('settings.modals.sysproxy.messages.durationTooShort')
      return
    }
    if (bypassError) {
      showNotice.error('settings.modals.sysproxy.messages.invalidBypass')
      return
    }
    if (!isValidProxyHost(value.proxy_host)) {
      showNotice.error('settings.modals.sysproxy.messages.invalidProxyHost')
      return
    }

    setSaving(true)
    setOpen(false)

    const { patch, needResetProxy } = buildSystemProxyPatch({
      value,
      current: {
        enable_proxy_guard,
        enable_bypass_check,
        proxy_guard_duration,
        system_proxy_bypass,
        proxy_auto_config,
        use_default_bypass,
        pac_file_content,
        proxy_host,
      },
      mixedPort: clashConfig?.mixedPort,
    })

    Promise.resolve().then(async () => {
      try {
        if (Object.keys(patch).length > 0) {
          mutateVerge({ ...verge, ...patch }, false)
          await patchVerge(patch)
        }

        setTimeout(async () => {
          try {
            await invalidateProxyState()

            if (needResetProxy && enabled) {
              const [currentSysProxy, currentAutoProxy] = await Promise.all([
                getSystemProxy(),
                getAutotemProxy(),
              ])

              const isProxyActive = value.pac
                ? currentAutoProxy?.enable
                : currentSysProxy?.enable

              if (isProxyActive) {
                await patchVergeConfig({ enable_system_proxy: false })
                await sleep(50)
                await patchVergeConfig({ enable_system_proxy: true })
                await invalidateProxyState()
              }
            }
          } catch (error) {
            console.warn('代理状态刷新失败:', error)
          }
        }, 50)
      } catch (error) {
        console.error('系统代理配置保存失败:', error)
        mutateVerge()
        showNotice.error(error)
      } finally {
        setSaving(false)
      }
    })
  })

  return (
    <SystemProxyUI
      open={open}
      saving={saving}
      enabled={enabled ?? false}
      value={value}
      isProxyReallyEnabled={isProxyReallyEnabled}
      getSystemProxyAddress={systemProxyAddressText}
      getCurrentPacUrl={currentPacUrl}
      bypassError={bypassError}
      separator={separator}
      hostOptions={hostOptions}
      editorOpen={editorOpen}
      pacEditorValue={pacEditorValue}
      pacEditorSavedValue={pacEditorSavedValue}
      defaultBypass={defaultBypass}
      onClose={() => setOpen(false)}
      onSave={onSave}
      setValue={setValue}
      openPacEditor={openPacEditor}
      setEditorOpen={setEditorOpen}
      setPacEditorValue={setPacEditorValue}
      handleSavePac={handleSavePac}
    />
  )
})
