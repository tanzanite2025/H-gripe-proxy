import { open } from '@tauri-apps/plugin-dialog'
import { Copy as ContentCopyRounded } from 'lucide-react'
import { useCallback, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { DialogRef, TooltipIcon } from '@/components/base'
import { Box } from '@/components/tailwind/Box'
import { Button } from '@/components/tailwind/Button'
import { MenuItem, Select } from '@/components/tailwind/Select'
import { TextField } from '@/components/tailwind/TextField'
import { useVerge } from '@/hooks/system'
import { updateLastCheckTime } from '@/hooks/system/use-update'
import { navItems } from '@/pages/_core/router'
import {
  authorizeStartupScript,
  clearStartupScriptAuthorization,
  copyClashEnv,
  openAppDir,
  openCoreDir,
  openDevTools,
} from '@/services/cmds'
import { supportedLanguages } from '@/services/i18n'
import { showNotice } from '@/services/notice-service'
import { checkUpdateSafe as checkUpdate } from '@/services/update'
import getSystem from '@/utils/misc'
import { version } from '@root/package.json'

import { BackupViewer } from './components/backup/backup-main'
import { HotkeyViewer } from './components/hotkey/hotkey-config'
import { ConfigViewer } from './components/misc/config-editor'
import { MiscViewer } from './components/misc/misc-config'
import { UpdateViewer } from './components/misc/update-config'
import { GuardState } from './components/proxy/guard-state'
import { SettingItem, SettingList } from './components/shared/setting-item'

interface Props {
  onError?: (err: Error) => void
}

const OS = getSystem()

const languageOptions = supportedLanguages.map((code) => {
  const labels: { [key: string]: string } = {
    en: 'English',
    ru: 'Русский',
    zh: '中文',
    fa: 'فارسی',
    tt: 'Татар',
    id: 'Bahasa Indonesia',
    ar: 'العربية',
    ko: '한국어',
    tr: 'Türkçe',
    de: 'Deutsch',
    es: 'Español',
    jp: '日本語',
    zhtw: '繁體中文',
  }
  const label = labels[code] || code
  return { code, label }
})

const SettingVergeBasic = ({ onError }: Props) => {
  const { t } = useTranslation()

  const { verge, patchVerge, mutateVerge } = useVerge()
  const {
    theme_mode,
    language,
    env_type,
    startup_script,
    start_page,
  } = verge ?? {}
  const configRef = useRef<DialogRef>(null)
  const hotkeyRef = useRef<DialogRef>(null)
  const miscRef = useRef<DialogRef>(null)
  const updateRef = useRef<DialogRef>(null)
  const backupRef = useRef<DialogRef>(null)

  const canOpenDevTools = import.meta.env.DEV

  const onChangeData = (patch: any) => {
    mutateVerge({ ...verge, ...patch }, false)
  }

  const onCopyClashEnv = useCallback(async () => {
    await copyClashEnv()
    showNotice.success('shared.feedback.notifications.common.copySuccess', 1000)
  }, [])

  const onCheckUpdate = async () => {
    try {
      const info = await checkUpdate()
      updateLastCheckTime()
      if (!info?.available) {
        showNotice.success(
          'settings.components.verge.advanced.notifications.latestVersion',
        )
      } else {
        updateRef.current?.open()
      }
    } catch (err: any) {
      showNotice.error(err)
    }
  }

  const copyVersion = useCallback(() => {
    navigator.clipboard.writeText(`v${version}`).then(() => {
      showNotice.success(
        'settings.components.verge.advanced.notifications.versionCopied',
        1000,
      )
    })
  }, [])

  return (
    <SettingList title={t('settings.components.verge.basic.title')}>
      <ConfigViewer ref={configRef} />
      <HotkeyViewer ref={hotkeyRef} />
      <MiscViewer ref={miscRef} />
      <UpdateViewer ref={updateRef} />
      <BackupViewer ref={backupRef} />

      <SettingItem label={t('settings.components.verge.basic.fields.language')}>
        <GuardState
          value={language ?? 'en'}
          onCatch={onError}
          onFormat={(e: any) => e.target.value}
          onChange={(e) => onChangeData({ language: e })}
          onGuard={(e) => patchVerge({ language: e })}
        >
          <Select size="small" className="w-[140px]">
            {languageOptions.map(({ code, label }) => (
              <MenuItem key={code} value={code}>
                {label}
              </MenuItem>
            ))}
          </Select>
        </GuardState>
      </SettingItem>


      <SettingItem
        label={t('settings.components.verge.basic.fields.copyEnvType')}
        extra={
          <TooltipIcon icon={ContentCopyRounded} onClick={onCopyClashEnv} />
        }
      >
        <GuardState
          value={env_type ?? (OS === 'windows' ? 'powershell' : 'bash')}
          onCatch={onError}
          onFormat={(e: any) => e.target.value}
          onChange={(e) => onChangeData({ env_type: e })}
          onGuard={(e) => patchVerge({ env_type: e })}
        >
          <Select size="small" className="w-[140px]">
            <MenuItem value="bash">Bash</MenuItem>
            <MenuItem value="fish">Fish</MenuItem>
            <MenuItem value="nushell">Nushell</MenuItem>
            <MenuItem value="cmd">CMD</MenuItem>
            <MenuItem value="powershell">PowerShell</MenuItem>
          </Select>
        </GuardState>
      </SettingItem>

      <SettingItem
        label={t('settings.components.verge.basic.fields.startPage')}
      >
        <GuardState
          value={start_page ?? '/'}
          onCatch={onError}
          onFormat={(e: any) => e.target.value}
          onChange={(e) => onChangeData({ start_page: e })}
          onGuard={(e) => patchVerge({ start_page: e })}
        >
          <Select size="small" className="w-[140px]">
            {navItems.map((page: { label: string; path: string }) => {
              return (
                <MenuItem key={page.path} value={page.path}>
                  {t(page.label)}
                </MenuItem>
              )
            })}
          </Select>
        </GuardState>
      </SettingItem>

      <SettingItem
        label={t('settings.components.verge.basic.fields.startupScript')}
      >
        <Box className="flex items-center gap-2">
          <TextField
            value={startup_script ?? ''}
            disabled
            readOnly
            className="w-[230px]"
          />
          <Button
            onClick={async () => {
              const selected = await open({
                directory: false,
                multiple: false,
                filters: [
                  {
                    name: 'Shell Script',
                    extensions: ['sh', 'bat', 'ps1'],
                  },
                ],
              })
              if (selected) {
                const scriptPath = `${selected}`
                try {
                  await authorizeStartupScript(scriptPath)
                  onChangeData({ startup_script: scriptPath })
                  await patchVerge({ startup_script: scriptPath })
                } catch (err: any) {
                  await clearStartupScriptAuthorization().catch(() => {})
                  showNotice.error(err)
                }
              }
            }}
          >
            {t('settings.components.verge.basic.actions.browse')}
          </Button>
          {startup_script && (
            <Button
              onClick={async () => {
                try {
                  await clearStartupScriptAuthorization()
                  onChangeData({ startup_script: '' })
                  await patchVerge({ startup_script: '' })
                } catch (err: any) {
                  showNotice.error(err)
                }
              }}
            >
              {t('shared.actions.clear')}
            </Button>
          )}
        </Box>
      </SettingItem>

      <SettingItem
        onClick={() => miscRef.current?.open()}
        label={t('settings.components.verge.basic.fields.misc')}
      />

      <SettingItem
        onClick={() => hotkeyRef.current?.open()}
        label={t('settings.components.verge.basic.fields.hotkeySetting')}
      />

      <SettingItem
        onClick={() => backupRef.current?.open()}
        label={t('settings.components.verge.advanced.fields.backupSetting')}
      />

      <SettingItem
        onClick={() => configRef.current?.open()}
        label={t('settings.components.verge.advanced.fields.runtimeConfig')}
      />

      <SettingItem
        onClick={openAppDir}
        label={t('settings.components.verge.advanced.fields.openConfDir')}
      />

      <SettingItem
        onClick={openCoreDir}
        label={t('settings.components.verge.advanced.fields.openCoreDir')}
      />

      <SettingItem
        onClick={onCheckUpdate}
        label={t('settings.components.verge.advanced.fields.checkUpdates')}
      />

      {canOpenDevTools && (
        <SettingItem
          onClick={openDevTools}
          label={t('settings.components.verge.advanced.fields.openDevTools')}
        />
      )}

      <SettingItem
        label={t('settings.components.verge.advanced.fields.vergeVersion')}
        extra={
          <TooltipIcon
            icon={ContentCopyRounded}
            onClick={copyVersion}
            title={t('settings.components.verge.advanced.actions.copyVersion')}
          />
        }
      >
        <div className="py-[7px] pr-1">v{version}</div>
      </SettingItem>
    </SettingList>
  )
}

export default SettingVergeBasic
