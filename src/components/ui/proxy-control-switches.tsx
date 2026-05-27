import { useLockFn } from 'ahooks'
import React, { useCallback, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { DialogRef } from '@/components/base'
import { SysproxyViewer } from '@/components/setting/components/proxy/system-proxy'
import { TunViewer } from '@/components/setting/components/network/tun-config'
import {
  useServiceInstaller,
  useServiceUninstaller,
  useSystemProxyState,
  useSystemState,
  useVerge,
} from '@/hooks/system'
import { showNotice } from '@/services/notice-service'
import { SwitchRow, TunModeExtraIcons } from './proxy-control-switches-ui'

interface ProxySwitchProps {
  label?: string
  onError?: (err: Error) => void
  noRightPadding?: boolean
}

const ProxyControlSwitches = ({
  label,
  onError,
  noRightPadding = false,
}: ProxySwitchProps) => {
  const { t } = useTranslation()
  const { verge, mutateVerge, patchVerge } = useVerge()
  const { installServiceAndRestartCore } = useServiceInstaller()
  const { uninstallServiceAndRestartCore } = useServiceUninstaller()
  const { indicator: systemProxyIndicator, toggleSystemProxy } =
    useSystemProxyState()
  const { isServiceOk, isTunModeAvailable, mutateSystemState } =
    useSystemState()

  const sysproxyRef = useRef<DialogRef>(null)
  const tunRef = useRef<DialogRef>(null)

  const { enable_tun_mode } = verge ?? {}

  const showErrorNotice = useCallback(
    (msg: string) => showNotice.error(msg),
    [],
  )

  const handleTunToggle = async (value: boolean) => {
    if (!isTunModeAvailable) {
      const msgKey = 'settings.sections.proxyControl.tooltips.tunUnavailable'
      showErrorNotice(msgKey)
      throw new Error(t(msgKey))
    }
    mutateVerge({ ...verge, enable_tun_mode: value }, false)
    await patchVerge({ enable_tun_mode: value })
  }

  const onInstallService = useLockFn(async () => {
    try {
      await installServiceAndRestartCore()
      await mutateSystemState()
    } catch (err) {
      showNotice.error(err)
    }
  })

  const onUninstallService = useLockFn(async () => {
    try {
      if (verge?.enable_tun_mode) {
        await handleTunToggle(false)
      }
      await uninstallServiceAndRestartCore()
      await mutateSystemState()
    } catch (err) {
      showNotice.error(err)
    }
  })

  const isSystemProxyMode =
    label === t('settings.sections.system.toggles.systemProxy') || !label
  const isTunMode = label === t('settings.sections.system.toggles.tunMode')

  const rows = (
    <>
      {isSystemProxyMode && (
        <SwitchRow
          label={t('settings.sections.proxyControl.fields.systemProxy')}
          active={systemProxyIndicator}
          infoTitle={t('settings.sections.proxyControl.tooltips.systemProxy')}
          onInfoClick={() => sysproxyRef.current?.open()}
          onToggle={(value) => toggleSystemProxy(value)}
          onError={onError}
          highlight={systemProxyIndicator}
          compact={noRightPadding}
        />
      )}

      {isTunMode && (
        <SwitchRow
          label={t('settings.sections.proxyControl.fields.tunMode')}
          active={enable_tun_mode || false}
          infoTitle={t('settings.sections.proxyControl.tooltips.tunMode')}
          onInfoClick={() => tunRef.current?.open()}
          onToggle={handleTunToggle}
          onError={onError}
          disabled={!isTunModeAvailable}
          highlight={enable_tun_mode || false}
          compact={noRightPadding}
          extraIcons={
            <TunModeExtraIcons
              isTunModeAvailable={isTunModeAvailable}
              isServiceOk={isServiceOk}
              tunUnavailableTooltip={t(
                'settings.sections.proxyControl.tooltips.tunUnavailable',
              )}
              installServiceTooltip={t(
                'settings.sections.proxyControl.actions.installService',
              )}
              uninstallServiceTooltip={t(
                'settings.sections.proxyControl.actions.uninstallService',
              )}
              onInstallService={onInstallService}
              onUninstallService={onUninstallService}
            />
          }
        />
      )}
    </>
  )

  if (noRightPadding) {
    return (
      <div className="w-full pr-2">
        {rows}
        <SysproxyViewer ref={sysproxyRef} />
        <TunViewer ref={tunRef} />
      </div>
    )
  }

  return (
    <>
      {rows}
      <SysproxyViewer ref={sysproxyRef} />
      <TunViewer ref={tunRef} />
    </>
  )
}

export default ProxyControlSwitches