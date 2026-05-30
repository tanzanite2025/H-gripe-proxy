import { useLockFn } from 'ahooks'
import {
  Info,
  Settings,
  ShieldCheck,
  Plug,
  Puzzle,
} from 'lucide-react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'

import { Chip } from '@/components/tailwind/Chip'
import { IconButton } from '@/components/tailwind/IconButton'
import {
  useServiceInstaller,
  useSystemState,
  useUpdate,
  useVerge,
} from '@/hooks/system'
import { updateLastCheckTime, readLastCheckTime } from '@/hooks/system/use-update'
import { getSystemInfo } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { version as appVersion } from '@root/package.json'

import { EnhancedCard } from './enhanced-card'

export const SystemInfoCard = () => {
  const { t } = useTranslation()
  const { verge, patchVerge } = useVerge()
  const navigate = useNavigate()
  const { isAdminMode, isSidecarMode, isNotRunningMode } = useSystemState()
  const { installServiceAndRestartCore } = useServiceInstaller()

  // 自动检查更新逻辑（lastCheckUpdate 由 useUpdate 统一管理）
  const { checkUpdate: triggerCheckUpdate, lastCheckUpdate } = useUpdate(true)

  const [osInfo, setOsInfo] = useState('')

  const lastCheckUpdateText = useMemo(
    () => (lastCheckUpdate ? new Date(lastCheckUpdate).toLocaleString() : '-'),
    [lastCheckUpdate],
  )

  // 初始化系统信息
  useEffect(() => {
    getSystemInfo()
      .then((info) => {
        const lines = info.split('\n')
        if (lines.length > 0) {
          const sysName = lines[0].split(': ')[1] || ''
          let sysVersion = lines[1].split(': ')[1] || ''

          if (
            sysName &&
            sysVersion.toLowerCase().startsWith(sysName.toLowerCase())
          ) {
            sysVersion = sysVersion.substring(sysName.length).trim()
          }

          setOsInfo(`${sysName} ${sysVersion}`)
        }
      })
      .catch(console.error)
  }, [])

  // 如果启用了自动检查更新但没有记录，设置当前时间并延迟检查
  useEffect(() => {
    if (!verge?.auto_check_update) return
    if (readLastCheckTime() !== null) return

    updateLastCheckTime()
    const timeoutId = window.setTimeout(() => {
      triggerCheckUpdate().catch(console.error)
    }, 5000)
    return () => window.clearTimeout(timeoutId)
  }, [verge?.auto_check_update, triggerCheckUpdate])

  // 导航到设置页面
  const goToSettings = useCallback(() => {
    navigate('/settings')
  }, [navigate])

  // 切换自启动状态
  const toggleAutoLaunch = useCallback(async () => {
    if (!verge) return
    try {
      await patchVerge({ enable_auto_launch: !verge.enable_auto_launch })
    } catch (err) {
      console.error('切换开机自启动状态失败:', err)
    }
  }, [verge, patchVerge])

  // 点击运行模式处理,Sidecar或纯管理员模式允许安装服务
  const handleRunningModeClick = useCallback(() => {
    if (isSidecarMode || (isAdminMode && isSidecarMode)) {
      installServiceAndRestartCore()
    }
  }, [isSidecarMode, isAdminMode, installServiceAndRestartCore])

  // 检查更新
  const onCheckUpdate = useLockFn(async () => {
    try {
      const result = await triggerCheckUpdate()
      const info = result.data
      if (!info?.available) {
        showNotice.success(
          'settings.components.verge.advanced.notifications.latestVersion',
        )
      } else {
        showNotice.info('shared.feedback.notifications.updateAvailable', 2000)
        goToSettings()
      }
    } catch (err) {
      showNotice.error(err)
    }
  })

  // 是否启用自启动
  const autoLaunchEnabled = useMemo(
    () => verge?.enable_auto_launch || false,
    [verge],
  )

  // 获取模式图标和文本
  const getModeIcon = () => {
    if (isNotRunningMode) {
      return (
        <span title={t('shared.statuses.disabled')}>
          <Puzzle className="text-warning text-base" />
        </span>
      )
    }

    if (isAdminMode) {
      // 判断是否为组合模式（管理员+服务）
      if (!isSidecarMode) {
        return (
          <>
            <span title={t('home.components.systemInfo.badges.adminMode')}>
              <ShieldCheck className="text-primary text-base" />
            </span>
            <span title={t('home.components.systemInfo.badges.serviceMode')}>
              <Plug className="text-success text-base ml-1" />
            </span>
          </>
        )
      }
      return (
        <span title={t('home.components.systemInfo.badges.adminMode')}>
          <ShieldCheck className="text-primary text-base" />
        </span>
      )
    } else if (isSidecarMode) {
      return (
        <span title={t('home.components.systemInfo.badges.sidecarMode')}>
          <Puzzle className="text-info text-base" />
        </span>
      )
    } else {
      return (
        <span title={t('home.components.systemInfo.badges.serviceMode')}>
          <Plug className="text-success text-base" />
        </span>
      )
    }
  }

  // 获取模式文本
  const getModeText = () => {
    if (isNotRunningMode) {
      return t('shared.statuses.disabled')
    }

    if (isAdminMode) {
      // 判断是否同时处于服务模式
      if (!isSidecarMode) {
        return t('home.components.systemInfo.badges.adminServiceMode')
      }
      return t('home.components.systemInfo.badges.adminMode')
    } else if (isSidecarMode) {
      return t('home.components.systemInfo.badges.sidecarMode')
    } else {
      return t('home.components.systemInfo.badges.serviceMode')
    }
  }

  // 只有当verge存在时才渲染内容
  if (!verge) return null

  return (
    <EnhancedCard
      title={t('home.components.systemInfo.title')}
      icon={<Info className="h-5 w-5" />}
      iconColor="error"
      fixedHeight={340}
      action={
        <IconButton
          size="small"
          onClick={goToSettings}
          title={t('home.components.systemInfo.actions.settings')}
        >
          <Settings className="h-4 w-4" />
        </IconButton>
      }
    >
      <div className="space-y-3">
        <div className="flex justify-between">
          <p className="text-sm text-text-secondary">
            {t('home.components.systemInfo.fields.osInfo')}
          </p>
          <p className="text-sm font-medium">
            {osInfo}
          </p>
        </div>
        <div className="border-t border-divider" />
        <div className="flex justify-between items-center">
          <p className="text-sm text-text-secondary">
            {t('home.components.systemInfo.fields.autoLaunch')}
          </p>
          <div className="flex items-center gap-2">
            <Chip
              size="small"
              label={
                autoLaunchEnabled
                  ? t('shared.statuses.enabled')
                  : t('shared.statuses.disabled')
              }
              color={autoLaunchEnabled ? 'success' : 'default'}
              variant={autoLaunchEnabled ? 'filled' : 'outlined'}
              onClick={toggleAutoLaunch}
              className="cursor-pointer"
            />
          </div>
        </div>
        <div className="border-t border-divider" />
        <div className="flex justify-between items-center">
          <p className="text-sm text-text-secondary">
            {t('home.components.systemInfo.fields.runningMode')}
          </p>
          <p
            className={`text-sm font-medium flex items-center gap-1 ${
              isSidecarMode || (isAdminMode && isSidecarMode)
                ? 'cursor-pointer underline hover:opacity-70'
                : 'cursor-default'
            }`}
            onClick={handleRunningModeClick}
          >
            {getModeIcon()}
            {getModeText()}
          </p>
        </div>
        <div className="border-t border-divider" />
        <div className="flex justify-between">
          <p className="text-sm text-text-secondary">
            {t('home.components.systemInfo.fields.lastCheckUpdate')}
          </p>
          <p
            className="text-sm font-medium cursor-pointer underline hover:opacity-70"
            onClick={onCheckUpdate}
          >
            {lastCheckUpdateText}
          </p>
        </div>
        <div className="border-t border-divider" />
        <div className="flex justify-between">
          <p className="text-sm text-text-secondary">
            {t('home.components.systemInfo.fields.vergeVersion')}
          </p>
          <p className="text-sm font-medium">
            v{appVersion}
          </p>
        </div>
      </div>
    </EnhancedCard>
  )
}
