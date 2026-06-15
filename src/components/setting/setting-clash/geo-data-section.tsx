import type { RefObject } from 'react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { DialogRef, Switch } from '@/components/base'
import { MenuItem, Select } from '@/components/tailwind/Select'
import { showNotice } from '@/services/notice-service'

import { GuardState } from '../components/proxy/guard-state'
import { SettingItem } from '../components/shared/setting-item'

import { SettingActionButton } from './action-button'
import {
  GEO_UPDATE_INTERVAL_OPTIONS,
  loadGeoSettings,
  saveGeoAutoUpdate,
  saveGeoUpdateInterval,
  triggerGeoUpdate,
} from './geo-data'

interface GeoDataSectionProps {
  geoSourceRef: RefObject<DialogRef | null>
  onError: (error: Error) => void
}

export function GeoDataSection({
  geoSourceRef,
  onError,
}: GeoDataSectionProps) {
  const { t } = useTranslation()
  const [loading, setLoading] = useState(true)
  const [updating, setUpdating] = useState(false)
  const [autoUpdate, setAutoUpdate] = useState(false)
  const [updateInterval, setUpdateInterval] = useState(24)
  const [lastUpdate, setLastUpdate] = useState('')

  useEffect(() => {
    let disposed = false

    const syncGeoSettings = async () => {
      try {
        const nextState = await loadGeoSettings()
        if (disposed) return

        setAutoUpdate(nextState.autoUpdate)
        setUpdateInterval(nextState.interval)
        setLastUpdate(nextState.lastUpdateLabel)
      } catch {
        // Keep fallback state when the base config cannot be read.
      } finally {
        if (!disposed) {
          setLoading(false)
        }
      }
    }

    void syncGeoSettings()

    return () => {
      disposed = true
    }
  }, [])

  const handleRefresh = async () => {
    setUpdating(true)
    try {
      const nextLastUpdate = await triggerGeoUpdate()
      setLastUpdate(nextLastUpdate)
      showNotice.success('settings.feedback.notifications.clash.geoDataUpdated')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setUpdating(false)
    }
  }

  const handleToggleAutoUpdate = async (enabled: boolean) => {
    setAutoUpdate(enabled)
    try {
      await saveGeoAutoUpdate(enabled)
    } catch (error) {
      showNotice.error(error)
    }
  }

  const handleChangeInterval = async (hours: number) => {
    setUpdateInterval(hours)
    try {
      await saveGeoUpdateInterval(hours)
    } catch (error) {
      showNotice.error(error)
    }
  }

  return (
    <>
      <SettingItem
        label={t('settings.sections.clash.form.fields.updateGeoData')}
        extra={
          <>
            <SettingActionButton onClick={handleRefresh} disabled={updating}>
              {updating ? '更新中...' : t('shared.actions.refresh')}
            </SettingActionButton>
            <SettingActionButton onClick={() => geoSourceRef.current?.open()}>
              数据源
            </SettingActionButton>
          </>
        }
      >
        <div className="flex items-center gap-2">
          <GuardState
            value={autoUpdate}
            valueProps="checked"
            onCatch={onError}
            onFormat={(_event: unknown, checked: boolean) => checked}
            onChange={(enabled) => setAutoUpdate(enabled)}
            onGuard={(enabled) => handleToggleAutoUpdate(enabled)}
          >
            <Switch checked={autoUpdate} disabled={loading} />
          </GuardState>
          {lastUpdate && (
            <span className="text-xs text-text-secondary">{lastUpdate}</span>
          )}
        </div>
      </SettingItem>

      {autoUpdate && (
        <SettingItem label="GeoData 更新间隔 (小时)">
          <div className="w-[100px]">
            <Select
              size="small"
              value={String(updateInterval)}
              onChange={(event: any) =>
                void handleChangeInterval(Number(event.target.value))
              }
            >
              {GEO_UPDATE_INTERVAL_OPTIONS.map((option) => (
                <MenuItem key={option} value={String(option)}>
                  {option === 168 ? '168 (每周)' : option}
                </MenuItem>
              ))}
            </Select>
          </div>
        </SettingItem>
      )}
    </>
  )
}
