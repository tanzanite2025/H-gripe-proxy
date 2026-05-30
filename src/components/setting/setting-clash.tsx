import { useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { updateGeo } from 'tauri-plugin-mihomo-api'

import { DialogRef, Switch } from '@/components/base'
import { MenuItem, Select } from '@/components/tailwind/Select'
import { useClash } from '@/hooks/data'
import { useClashLog, useVerge } from '@/hooks/system'
import { invoke_uwp_tool } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import getSystem from '@/utils/misc'

import { ClashCoreViewer } from './components/clash/clash-core'
import { ClashPortViewer } from './components/clash/clash-port'
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
  } = clash ?? {}

  const { verge_mixed_port } = verge ?? {}

  const webRef = useRef<DialogRef>(null)
  const portRef = useRef<DialogRef>(null)
  const ctrlRef = useRef<DialogRef>(null)
  const coreRef = useRef<DialogRef>(null)
  const networkRef = useRef<DialogRef>(null)
  const corsRef = useRef<DialogRef>(null)
  const tunnelRef = useRef<DialogRef>(null)

  const onSwitchFormat = (_e: any, value: boolean) => value
  const onChangeData = (patch: Partial<IConfigData>) => {
    mutateClash((old) => ({ ...old!, ...patch }), false)
  }
  const onUpdateGeo = async () => {
    try {
      await updateGeo()
      showNotice.success('settings.feedback.notifications.clash.geoDataUpdated')
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
          <button
            type="button"
            className="text-xs px-3 py-0.5 rounded-full border border-border text-text-secondary whitespace-nowrap hover:bg-white/5 cursor-pointer transition-colors"
            onClick={onUpdateGeo}
          >
            刷新
          </button>
        }
      >
        <span />
      </SettingItem>

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
