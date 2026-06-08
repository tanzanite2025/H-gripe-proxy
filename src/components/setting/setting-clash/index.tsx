import { useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { DialogRef, Switch } from '@/components/base'
import { MenuItem, Select } from '@/components/tailwind/Select'
import { useClash } from '@/hooks/data'
import { useClashLog } from '@/hooks/system'
import { invoke_uwp_tool } from '@/services/cmds'
import getSystem from '@/utils/misc'

import { ClashPortViewer } from '../components/clash/clash-port'
import { GeoSourceConfig } from '../components/clash/geo-source-config'
import { ControllerViewer } from '../components/network/controller'
import { HeaderConfiguration } from '../components/network/external-cors'
import { NetworkInterfaceViewer } from '../components/network/network-interface'
import { TunnelsViewer } from '../components/network/tunnels-config'
import { GuardState } from '../components/proxy/guard-state'
import { SettingItem, SettingList } from '../components/shared/setting-item'
import { SettingActionButton } from './action-button'
import { GeoDataSection } from './geo-data-section'

const isWindows = getSystem() === 'windows'

interface Props {
  onError: (err: Error) => void
}

const formatSwitchValue = (_event: unknown, value: boolean) => value

const SettingClash = ({ onError }: Props) => {
  const { t } = useTranslation()
  const { clash, mutateClash, patchClash } = useClash()
  const [, setClashLog] = useClashLog()

  const {
    ipv6,
    'allow-lan': allowLan,
    'log-level': logLevel,
    'unified-delay': unifiedDelay,
    'find-process-mode': findProcessMode,
  } = clash ?? {}

  const portRef = useRef<DialogRef>(null)
  const ctrlRef = useRef<DialogRef>(null)
  const geoSourceRef = useRef<DialogRef>(null)
  const networkRef = useRef<DialogRef>(null)
  const corsRef = useRef<DialogRef>(null)
  const tunnelRef = useRef<DialogRef>(null)

  const onChangeData = (patch: Partial<IConfigData>) => {
    mutateClash((old) => ({ ...old!, ...patch }), false)
  }

  return (
    <SettingList title={t('settings.sections.clash.title')}>
      <ClashPortViewer ref={portRef} />
      <ControllerViewer ref={ctrlRef} />
      <GeoSourceConfig ref={geoSourceRef} />
      <NetworkInterfaceViewer ref={networkRef} />
      <HeaderConfiguration ref={corsRef} />
      <TunnelsViewer ref={tunnelRef} />

      <SettingItem
        label={t('settings.sections.clash.form.fields.allowLan')}
        extra={
          <SettingActionButton onClick={() => networkRef.current?.open()}>
            接口
          </SettingActionButton>
        }
      >
        <GuardState
          value={allowLan ?? false}
          valueProps="checked"
          onCatch={onError}
          onFormat={formatSwitchValue}
          onChange={(value) => onChangeData({ 'allow-lan': value })}
          onGuard={(value) => patchClash({ 'allow-lan': value })}
        >
          <Switch checked={allowLan ?? false} />
        </GuardState>
      </SettingItem>

      <SettingItem label={t('settings.sections.clash.form.fields.ipv6')}>
        <GuardState
          value={ipv6 ?? false}
          valueProps="checked"
          onCatch={onError}
          onFormat={formatSwitchValue}
          onChange={(value) => onChangeData({ ipv6: value })}
          onGuard={(value) => patchClash({ ipv6: value })}
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
          onFormat={formatSwitchValue}
          onChange={(value) => onChangeData({ 'unified-delay': value })}
          onGuard={(value) => patchClash({ 'unified-delay': value })}
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
          onFormat={(event: any) => event.target.value}
          onChange={(value) => onChangeData({ 'find-process-mode': value })}
          onGuard={(value) => patchClash({ 'find-process-mode': value })}
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

      <SettingItem label={t('settings.sections.clash.form.fields.logLevel')}>
        <GuardState
          value={logLevel === 'warn' ? 'warning' : (logLevel ?? 'info')}
          onCatch={onError}
          onFormat={(event: any) => event.target.value}
          onChange={(value) => onChangeData({ 'log-level': value })}
          onGuard={(value) => {
            setClashLog((old) => ({ ...old!, logLevel: value }))
            return patchClash({ 'log-level': value })
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
          <SettingActionButton onClick={() => portRef.current?.open()}>
            设置
          </SettingActionButton>
        }
      >
        <span />
      </SettingItem>

      <SettingItem
        label={t('settings.sections.clash.form.fields.external')}
        extra={
          <>
            <SettingActionButton onClick={() => ctrlRef.current?.open()}>
              控制器
            </SettingActionButton>
            <SettingActionButton onClick={() => corsRef.current?.open()}>
              跨域设置
            </SettingActionButton>
          </>
        }
      >
        <span />
      </SettingItem>

      {isWindows && (
        <SettingItem
          label={t('settings.sections.clash.form.fields.openUwpTool')}
          extra={
            <SettingActionButton onClick={invoke_uwp_tool}>
              打开
            </SettingActionButton>
          }
        >
          <span />
        </SettingItem>
      )}

      <GeoDataSection geoSourceRef={geoSourceRef} onError={onError} />

      <SettingItem
        label={t('settings.sections.clash.form.fields.tunnels.title')}
        extra={
          <SettingActionButton onClick={() => tunnelRef.current?.open()}>
            打开
          </SettingActionButton>
        }
      >
        <span />
      </SettingItem>
    </SettingList>
  )
}

export default SettingClash
