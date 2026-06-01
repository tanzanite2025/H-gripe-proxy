import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { getBaseConfig, patchBaseConfig, updateGeo } from 'tauri-plugin-mihomo-api'

import { DialogRef, Switch } from '@/components/base'
import { MenuItem, Select } from '@/components/tailwind/Select'
import { useClash } from '@/hooks/data'
import { useClashLog, useVerge } from '@/hooks/system'
import { invoke_uwp_tool, getGeoDataUpdateTime } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import getSystem from '@/utils/misc'

import { ClashCoreViewer } from './components/clash/clash-core'
import { ClashPortViewer } from './components/clash/clash-port'
import { GeoSourceConfig } from './components/clash/geo-source-config'
import { ControllerViewer } from './components/network/controller'
import { HeaderConfiguration } from './components/network/external-cors'
import { NetworkInterfaceViewer } from './components/network/network-interface'
import { TunnelsViewer } from './components/network/tunnels-config'
import { GuardState } from './components/proxy/guard-state'
import { SettingItem, SettingList } from './components/shared/setting-item'
import { WebUIViewer } from './components/webui/webui-config'

const isWIN = getSystem() === 'windows'

interface Props {
  onError: (err: Error) => void
}

const SettingClash = ({ onError }: Props) => {
  const { t } = useTranslation()

  const { clash, version, mutateClash, patchClash } = useClash()
  const { verge } = useVerge()
  const [, setClashLog] = useClashLog()

  const {
    ipv6,
    'allow-lan': allowLan,
    'log-level': logLevel,
    'unified-delay': unifiedDelay,
    'find-process-mode': findProcessMode,
  } = clash ?? {}

  const { verge_mixed_port } = verge ?? {}

  const webRef = useRef<DialogRef>(null)
  const portRef = useRef<DialogRef>(null)
  const ctrlRef = useRef<DialogRef>(null)
  const coreRef = useRef<DialogRef>(null)
  const geoSourceRef = useRef<DialogRef>(null)
  const networkRef = useRef<DialogRef>(null)
  const corsRef = useRef<DialogRef>(null)
  const tunnelRef = useRef<DialogRef>(null)

  const onSwitchFormat = (_e: any, value: boolean) => value
  const onChangeData = (patch: Partial<IConfigData>) => {
    mutateClash((old) => ({ ...old!, ...patch }), false)
  }
  const [geoUpdating, setGeoUpdating] = useState(false)
  const [geoAutoUpdate, setGeoAutoUpdate] = useState(false)
  const [geoUpdateInterval, setGeoUpdateInterval] = useState(24)
  const [geoLastUpdate, setGeoLastUpdate] = useState<string>('')

  // 加载 Geo 配置
  useEffect(() => {
    getBaseConfig().then((cfg) => {
      setGeoAutoUpdate(cfg.geoAutoUpdate)
      setGeoUpdateInterval(cfg.geoUpdateInterval)
    }).catch(() => {})
    getGeoDataUpdateTime().then((t) => {
      const latest = [t.mmdb, t.geoip, t.asn, t.geosite]
        .filter((v): v is number => v != null)
        .sort((a, b) => b - a)[0]
      if (latest) {
        const d = new Date(latest)
        const diff = Date.now() - d.getTime()
        const hours = Math.floor(diff / 3600000)
        const days = Math.floor(hours / 24)
        setGeoLastUpdate(
          days > 0 ? `${days} 天前` : hours > 0 ? `${hours} 小时前` : '刚刚'
        )
      }
    }).catch(() => {})
  }, [])

  const onUpdateGeo = async () => {
    setGeoUpdating(true)
    try {
      await updateGeo()
      showNotice.success('settings.feedback.notifications.clash.geoDataUpdated')
      // 刷新更新时间
      getGeoDataUpdateTime().then((t) => {
        const latest = [t.mmdb, t.geoip, t.asn, t.geosite]
          .filter((v): v is number => v != null)
          .sort((a, b) => b - a)[0]
        if (latest) {
          setGeoLastUpdate('刚刚')
        }
      }).catch(() => {})
    } catch (err: any) {
      showNotice.error(err)
    } finally {
      setGeoUpdating(false)
    }
  }

  const onToggleGeoAutoUpdate = async (val: boolean) => {
    setGeoAutoUpdate(val)
    try {
      await patchBaseConfig({ 'geo-auto-update': val })
    } catch (err: any) {
      showNotice.error(err)
    }
  }

  const onChangeGeoInterval = async (val: number) => {
    setGeoUpdateInterval(val)
    try {
      await patchBaseConfig({ 'geo-update-interval': val })
    } catch (err: any) {
      showNotice.error(err)
    }
  }

  return (
    <SettingList title={t('settings.sections.clash.title')}>
      <WebUIViewer ref={webRef} />
      <ClashPortViewer ref={portRef} />
      <ControllerViewer ref={ctrlRef} />
      <ClashCoreViewer ref={coreRef} />
      <GeoSourceConfig ref={geoSourceRef} />
      <NetworkInterfaceViewer ref={networkRef} />
      <HeaderConfiguration ref={corsRef} />
      <TunnelsViewer ref={tunnelRef} />
      <SettingItem
        label={t('settings.sections.clash.form.fields.allowLan')}
        extra={
          <button
            type="button"
            className="text-xs px-3 py-0.5 rounded-full border border-border text-text-secondary whitespace-nowrap hover:bg-white/5 cursor-pointer transition-colors"
            onClick={() => {
              networkRef.current?.open()
            }}
          >
            接口
          </button>
        }
      >
        <GuardState
          value={allowLan ?? false}
          valueProps="checked"
          onCatch={onError}
          onFormat={onSwitchFormat}
          onChange={(e) => onChangeData({ 'allow-lan': e })}
          onGuard={(e) => patchClash({ 'allow-lan': e })}
        >
          <Switch checked={allowLan ?? false} />
        </GuardState>
      </SettingItem>

      <SettingItem label={t('settings.sections.clash.form.fields.ipv6')}>
        <GuardState
          value={ipv6 ?? false}
          valueProps="checked"
          onCatch={onError}
          onFormat={onSwitchFormat}
          onChange={(e) => onChangeData({ ipv6: e })}
          onGuard={(e) => patchClash({ ipv6: e })}
        >
          <Switch checked={ipv6 ?? false} />
        </GuardState>
      </SettingItem>

      <SettingItem
        label={t('settings.sections.clash.form.fields.unifiedDelay')}
      >
        <GuardState
          value={unifiedDelay ?? false}
          valueProps="checked"
          onCatch={onError}
          onFormat={onSwitchFormat}
          onChange={(e) => onChangeData({ 'unified-delay': e })}
          onGuard={(e) => patchClash({ 'unified-delay': e })}
        >
          <Switch checked={unifiedDelay ?? false} />
        </GuardState>
      </SettingItem>

      <SettingItem
        label={t('settings.sections.clash.form.fields.findProcessMode')}
      >
        <GuardState
          value={findProcessMode ?? 'strict'}
          onCatch={onError}
          onFormat={(e: any) => e.target.value}
          onChange={(e) => onChangeData({ 'find-process-mode': e })}
          onGuard={(e) => patchClash({ 'find-process-mode': e })}
        >
          <div className="w-[100px]">
            <Select size="small">
              <MenuItem value="always">
                {t('settings.sections.clash.form.options.findProcessMode.always')}
              </MenuItem>
              <MenuItem value="strict">
                {t('settings.sections.clash.form.options.findProcessMode.strict')}
              </MenuItem>
              <MenuItem value="off">
                {t('settings.sections.clash.form.options.findProcessMode.off')}
              </MenuItem>
            </Select>
          </div>
        </GuardState>
      </SettingItem>

      <SettingItem
        label={t('settings.sections.clash.form.fields.logLevel')}
      >
        <GuardState
          value={logLevel === 'warn' ? 'warning' : (logLevel ?? 'info')}
          onCatch={onError}
          onFormat={(e: any) => e.target.value}
          onChange={(e) => onChangeData({ 'log-level': e })}
          onGuard={(e) => {
            setClashLog((pre) => ({ ...pre!, logLevel: e }))
            return patchClash({ 'log-level': e })
          }}
        >
          <div className="w-[100px]">
            <Select size="small">
              <MenuItem value="debug">
                {t('settings.sections.clash.form.options.logLevel.debug')}
              </MenuItem>
              <MenuItem value="info">
                {t('settings.sections.clash.form.options.logLevel.info')}
              </MenuItem>
              <MenuItem value="warning">
                {t('settings.sections.clash.form.options.logLevel.warning')}
              </MenuItem>
              <MenuItem value="error">
                {t('settings.sections.clash.form.options.logLevel.error')}
              </MenuItem>
              <MenuItem value="silent">
                {t('settings.sections.clash.form.options.logLevel.silent')}
              </MenuItem>
            </Select>
          </div>
        </GuardState>
      </SettingItem>

      <SettingItem
        label={t('settings.sections.clash.form.fields.portConfig')}
        extra={
          <button
            type="button"
            className="text-xs px-3 py-0.5 rounded-full border border-border text-text-secondary whitespace-nowrap hover:bg-white/5 cursor-pointer transition-colors"
            onClick={() => portRef.current?.open()}
          >
            设置
          </button>
        }
      >
        <span />
      </SettingItem>

      <SettingItem
        label={t('settings.sections.clash.form.fields.external')}
        extra={
          <>
            <button
              type="button"
              className="text-xs px-3 py-0.5 rounded-full border border-border text-text-secondary whitespace-nowrap hover:bg-white/5 cursor-pointer transition-colors"
              onClick={() => {
                ctrlRef.current?.open()
              }}
            >
              控制器
            </button>
            <button
              type="button"
              className="text-xs px-3 py-0.5 rounded-full border border-border text-text-secondary whitespace-nowrap hover:bg-white/5 cursor-pointer transition-colors"
              onClick={() => {
                corsRef.current?.open()
              }}
            >
              跨域设置
            </button>
          </>
        }
      >
        <span />
      </SettingItem>

      <SettingItem
        label={t('settings.sections.clash.form.fields.webUI')}
        extra={
          <button
            type="button"
            className="text-xs px-3 py-0.5 rounded-full border border-border text-text-secondary whitespace-nowrap hover:bg-white/5 cursor-pointer transition-colors"
            onClick={() => webRef.current?.open()}
          >
            打开
          </button>
        }
      >
        <span />
      </SettingItem>

      <SettingItem
        label={t('settings.sections.clash.form.fields.clashCore')}
        extra={
          <button
            type="button"
            className="text-xs px-3 py-0.5 rounded-full border border-border text-text-secondary whitespace-nowrap hover:bg-white/5 cursor-pointer transition-colors"
            onClick={() => coreRef.current?.open()}
          >
            设置
          </button>
        }
      >
        <div className="py-[7px] pr-1">{version}</div>
      </SettingItem>

      {isWIN && (
        <SettingItem
          label={t('settings.sections.clash.form.fields.openUwpTool')}
          extra={
            <button
              type="button"
              className="text-xs px-3 py-0.5 rounded-full border border-border text-text-secondary whitespace-nowrap hover:bg-white/5 cursor-pointer transition-colors"
              onClick={invoke_uwp_tool}
            >
              打开
            </button>
          }
        >
          <span />
        </SettingItem>
      )}

      <SettingItem
        label={t('settings.sections.clash.form.fields.updateGeoData')}
        extra={
          <>
            <button
              type="button"
              className="text-xs px-3 py-0.5 rounded-full border border-border text-text-secondary whitespace-nowrap hover:bg-white/5 cursor-pointer transition-colors disabled:opacity-50"
              onClick={onUpdateGeo}
              disabled={geoUpdating}
            >
              {geoUpdating ? '更新中...' : '刷新'}
            </button>
            <button
              type="button"
              className="text-xs px-3 py-0.5 rounded-full border border-border text-text-secondary whitespace-nowrap hover:bg-white/5 cursor-pointer transition-colors"
              onClick={() => geoSourceRef.current?.open()}
            >
              数据源
            </button>
          </>
        }
      >
        <div className="flex items-center gap-2">
          <GuardState
            value={geoAutoUpdate}
            valueProps="checked"
            onCatch={onError}
            onFormat={onSwitchFormat}
            onChange={(e) => onToggleGeoAutoUpdate(e)}
          >
            <Switch checked={geoAutoUpdate} />
          </GuardState>
          {geoLastUpdate && (
            <span className="text-xs text-text-secondary">{geoLastUpdate}</span>
          )}
        </div>
      </SettingItem>

      {geoAutoUpdate && (
        <SettingItem label="GeoData 更新间隔 (小时)">
          <div className="w-[100px]">
            <Select
              size="small"
              value={String(geoUpdateInterval)}
              onChange={(e: any) => onChangeGeoInterval(Number(e.target.value))}
            >
              <MenuItem value="6">6</MenuItem>
              <MenuItem value="12">12</MenuItem>
              <MenuItem value="24">24</MenuItem>
              <MenuItem value="48">48</MenuItem>
              <MenuItem value="72">72</MenuItem>
              <MenuItem value="168">168 (每周)</MenuItem>
            </Select>
          </div>
        </SettingItem>
      )}

      <SettingItem
        label={t('settings.sections.clash.form.fields.tunnels.title')}
        extra={
          <button
            type="button"
            className="text-xs px-3 py-0.5 rounded-full border border-border text-text-secondary whitespace-nowrap hover:bg-white/5 cursor-pointer transition-colors"
            onClick={() => tunnelRef.current?.open()}
          >
            打开
          </button>
        }
      >
        <span />
      </SettingItem>
    </SettingList>
  )
}

export default SettingClash
