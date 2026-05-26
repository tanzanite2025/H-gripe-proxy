import {
  BuildRounded,
  DeleteForeverRounded,
  PauseCircleOutlineRounded,
  PlayCircleOutlineRounded,
  SettingsRounded,
  WarningRounded,
} from '@mui/icons-material'
import { Box, Typography, alpha, useTheme } from '@mui/material'
import { useLockFn } from 'ahooks'
import React, { useCallback, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { DialogRef, Switch, TooltipIcon } from '@/components/base'
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

interface ProxySwitchProps {
  label?: string
  onError?: (err: Error) => void
  noRightPadding?: boolean
}

interface SwitchRowProps {
  label: string
  active: boolean
  disabled?: boolean
  infoTitle: string
  onInfoClick?: () => void
  extraIcons?: React.ReactNode
  onToggle: (value: boolean) => Promise<void>
  onError?: (err: Error) => void
  highlight?: boolean
  compact?: boolean
}

/**
 * 抽取的子组件：统一的开关 UI
 * active = 真实状态OS/配置 乐观更新
 */
const SwitchRow = ({
  label,
  active,
  disabled,
  infoTitle,
  onInfoClick,
  extraIcons,
  onToggle,
  onError,
  highlight,
  compact,
}: SwitchRowProps) => {
  const theme = useTheme()
  const [checked, setChecked] = useState(active)
  const pendingRef = useRef(false)

  if (pendingRef.current) {
    if (active === checked) pendingRef.current = false
  } else if (checked !== active) {
    setChecked(active)
  }

  const handleChange = (_: React.ChangeEvent, value: boolean) => {
    pendingRef.current = true
    setChecked(value)
    onToggle(value)
      .catch((err: any) => {
        setChecked(active)
        onError?.(err)
      })
      .finally(() => {
        pendingRef.current = false
      })
  }

  if (compact) {
    return (
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          p: 1,
          pr: 2,
          borderRadius: 1.5,
          bgcolor: highlight
            ? alpha(theme.palette.success.main, 0.07)
            : 'transparent',
          opacity: disabled ? 0.6 : 1,
          transition: 'background-color 0.3s',
        }}
      >
        <Box sx={{ display: 'flex', alignItems: 'center' }}>
          {active ? (
            <PlayCircleOutlineRounded sx={{ color: 'success.main', mr: 1 }} />
          ) : (
            <PauseCircleOutlineRounded sx={{ color: 'text.disabled', mr: 1 }} />
          )}
          <Typography
            className="uds-card-title"
            variant="subtitle1"
            sx={{ fontWeight: 500, fontSize: '15px' }}
          >
            {label}
          </Typography>
          <TooltipIcon
            title={infoTitle}
            icon={SettingsRounded}
            onClick={onInfoClick}
            sx={{ ml: 1 }}
          />
          {extraIcons}
        </Box>

        <Switch
          edge="end"
          disabled={disabled}
          checked={checked}
          onChange={handleChange}
        />
      </Box>
    )
  }

  return (
    <Box
      className="uds-settings-item"
      sx={{
        opacity: disabled ? 0.6 : 1,
      }}
    >
      <Box
        className="uds-settings-item__body"
        sx={{
          bgcolor: highlight ? alpha(theme.palette.success.main, 0.07) : 'transparent',
        }}
      >
        <Box className="uds-settings-item__main">
          <Box className="uds-settings-item__label-row">
            {active ? (
              <PlayCircleOutlineRounded sx={{ color: 'success.main' }} />
            ) : (
              <PauseCircleOutlineRounded sx={{ color: 'text.disabled' }} />
            )}
            <Typography
              className="uds-settings-item__label uds-card-title"
              variant="subtitle1"
              sx={{ fontWeight: 500, fontSize: '15px' }}
            >
              {label}
            </Typography>
            <TooltipIcon
              title={infoTitle}
              icon={SettingsRounded}
              onClick={onInfoClick}
            />
            {extraIcons}
          </Box>
        </Box>

        <Box className="uds-settings-item__control">
          <Switch
            edge="end"
            disabled={disabled}
            checked={checked}
            onChange={handleChange}
          />
        </Box>
      </Box>
    </Box>
  )
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
            <>
              {!isTunModeAvailable && (
                <>
                  <TooltipIcon
                    title={t(
                      'settings.sections.proxyControl.tooltips.tunUnavailable',
                    )}
                    icon={WarningRounded}
                    sx={{ color: 'warning.main', ml: 1 }}
                  />
                  <TooltipIcon
                    title={t(
                      'settings.sections.proxyControl.actions.installService',
                    )}
                    icon={BuildRounded}
                    color="primary"
                    onClick={onInstallService}
                    sx={{ ml: 1 }}
                  />
                </>
              )}
              {isServiceOk && (
                <TooltipIcon
                  title={t(
                    'settings.sections.proxyControl.actions.uninstallService',
                  )}
                  icon={DeleteForeverRounded}
                  color="secondary"
                  onClick={onUninstallService}
                  sx={{ ml: 1 }}
                />
              )}
            </>
          }
        />
      )}
    </>
  )

  if (noRightPadding) {
    return (
      <Box sx={{ width: '100%', pr: 1 }}>
        {rows}
        <SysproxyViewer ref={sysproxyRef} />
        <TunViewer ref={tunRef} />
      </Box>
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