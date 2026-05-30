import React from 'react'
import { useTranslation } from 'react-i18next'

import { Switch } from '@/components/base'
import ProxyControlSwitches from '@/components/ui/proxy-control-switches'
import { useVerge } from '@/hooks/system'

import { GuardState } from './components/proxy/guard-state'
import { SettingList, SettingItem } from './components/shared/setting-item'

interface Props {
  onError?: (err: Error) => void
}

const SettingSystem = ({ onError }: Props) => {
  const { t } = useTranslation()

  const { verge, mutateVerge, patchVerge } = useVerge()

  const { enable_auto_launch, enable_silent_start } = verge ?? {}

  const onSwitchFormat = (
    _e: React.ChangeEvent<HTMLInputElement>,
    value: boolean,
  ) => value
  const onChangeData = (patch: Partial<IVergeConfig>) => {
    mutateVerge({ ...verge, ...patch }, false)
  }

  return (
    <SettingList title={t('settings.sections.system.title')}>
      <ProxyControlSwitches
        label={t('settings.sections.system.toggles.tunMode')}
        onError={onError}
      />

      <ProxyControlSwitches
        label={t('settings.sections.system.toggles.systemProxy')}
        onError={onError}
      />

      <SettingItem label={t('settings.sections.system.fields.autoLaunch')}>
        <GuardState
          value={enable_auto_launch ?? false}
          valueProps="checked"
          onCatch={onError}
          onFormat={onSwitchFormat}
          onChange={(e) => {
            onChangeData({ enable_auto_launch: e })
          }}
          onGuard={async (e) => {
            try {
              // 閸忓牐袝閸欐叅I閺囧瓨鏌婄粩瀣祮閻鍩岄崣宥夘洯
              onChangeData({ enable_auto_launch: e })
              await patchVerge({ enable_auto_launch: e })
              return Promise.resolve()
            } catch (error) {
              // 婵″倹鐏夐崙娲晩閿涘本浠径宥呭斧婵濮搁幀?
              onChangeData({ enable_auto_launch: !e })
              return Promise.reject(error)
            }
          }}
        >
          <Switch />
        </GuardState>
      </SettingItem>

      <SettingItem
        label={t('settings.sections.system.fields.silentStart')}
      >
        <GuardState
          value={enable_silent_start ?? false}
          valueProps="checked"
          onCatch={onError}
          onFormat={onSwitchFormat}
          onChange={(e) => onChangeData({ enable_silent_start: e })}
          onGuard={(e) => patchVerge({ enable_silent_start: e })}
        >
          <Switch />
        </GuardState>
      </SettingItem>
    </SettingList>
  )
}

export default SettingSystem

