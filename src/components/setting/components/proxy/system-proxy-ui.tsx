import { Edit } from 'lucide-react'
import type { ChangeEvent, Dispatch, SetStateAction } from 'react'
import { useTranslation } from 'react-i18next'

import {
  BaseDialog,
  BaseFieldset,
  BaseSplitChipEditor,
  Switch,
  TooltipIcon,
} from '@/components/base'
import { EditorViewer } from '@/components/profile/editor-viewer'
import {
  Button,
  Chip,
  InputAdornment,
  List,
  ListItem,
  ListItemText,
  TextField,
} from '@/components/tailwind'

import { splitBypass } from './system-proxy/helpers'
import type { SystemProxyFormValue } from './system-proxy/types'

interface SystemProxyUIProps {
  open: boolean
  saving: boolean
  enabled: boolean
  value: SystemProxyFormValue
  isProxyReallyEnabled: boolean
  getSystemProxyAddress: string
  getCurrentPacUrl: string
  bypassError: boolean
  separator: string
  hostOptions: string[]
  editorOpen: boolean
  pacEditorValue: string
  pacEditorSavedValue: string
  defaultBypass: () => string
  onClose: () => void
  onSave: () => void
  setValue: Dispatch<SetStateAction<SystemProxyFormValue>>
  openPacEditor: () => void
  setEditorOpen: (open: boolean) => void
  setPacEditorValue: (value: string) => void
  handleSavePac: () => void
}

export const SystemProxyUI = ({
  open,
  saving,
  enabled,
  value,
  isProxyReallyEnabled,
  getSystemProxyAddress,
  getCurrentPacUrl,
  bypassError,
  separator,
  hostOptions,
  editorOpen,
  pacEditorValue,
  pacEditorSavedValue,
  defaultBypass,
  onClose,
  onSave,
  setValue,
  openPacEditor,
  setEditorOpen,
  setPacEditorValue,
  handleSavePac,
}: SystemProxyUIProps) => {
  const { t } = useTranslation()

  return (
    <BaseDialog
      open={open}
      title={t('settings.modals.sysproxy.title')}
      className="w-[450px] max-h-[565px]"
      okBtn={t('shared.actions.save')}
      cancelBtn={t('shared.actions.cancel')}
      onClose={onClose}
      onCancel={onClose}
      onOk={onSave}
      loading={saving}
      disableOk={saving}
    >
      <List>
        <BaseFieldset
          label={t('settings.modals.sysproxy.fieldsets.currentStatus')}
          padding="15px 10px"
        >
          <div className="mt-1 flex">
            <span className="flex-none">
              {t('settings.modals.sysproxy.fields.enableStatus')}
            </span>
            <span className="ml-auto">
              {isProxyReallyEnabled
                ? t('shared.statuses.enabled')
                : t('shared.statuses.disabled')}
            </span>
          </div>
          {!value.pac && (
            <div className="mt-1 flex">
              <span className="flex-none">
                {t('settings.modals.sysproxy.fields.serverAddr')}
              </span>
              <span className="ml-auto">{getSystemProxyAddress}</span>
            </div>
          )}
          {value.pac && (
            <div className="mt-1 flex">
              <span className="flex-none">
                {t('settings.modals.sysproxy.fields.pacUrl')}
              </span>
              <span className="ml-auto">{getCurrentPacUrl || '-'}</span>
            </div>
          )}
        </BaseFieldset>

        <ListItem className="px-0.5 py-1.5">
          <ListItemText
            primary={t('settings.modals.sysproxy.fields.proxyHost')}
          />
          <div className="w-[150px]">
            <select
              className="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-800"
              value={value.proxy_host}
              onChange={(e: ChangeEvent<HTMLSelectElement>) => {
                setValue((v) => ({
                  ...v,
                  proxy_host: e.target.value || '127.0.0.1',
                }))
              }}
            >
              {hostOptions.map((option) => (
                <option key={option} value={option}>
                  {option}
                </option>
              ))}
            </select>
          </div>
        </ListItem>

        <ListItem className="px-0.5 py-1.5">
          <ListItemText
            primary={t('settings.modals.sysproxy.fields.usePacMode')}
          />
          <Switch
            disabled={!enabled}
            checked={value.pac}
            onCheckedChange={(checked) => setValue((v) => ({ ...v, pac: checked }))}
          />
        </ListItem>

        <ListItem className="px-0.5 py-1.5">
          <ListItemText
            primary={t('settings.modals.sysproxy.fields.proxyGuard')}
            className="max-w-fit"
          />
          <TooltipIcon
            title={t('settings.modals.sysproxy.tooltips.proxyGuard')}
            className="opacity-70"
          />
          <Switch
            disabled={!enabled}
            checked={value.guard}
            onCheckedChange={(checked) => setValue((v) => ({ ...v, guard: checked }))}
            className="ml-auto"
          />
        </ListItem>

        <ListItem className="px-0.5 py-1.5">
          <ListItemText
            primary={t('settings.modals.sysproxy.fields.guardDuration')}
          />
          <TextField
            disabled={!enabled}
            value={value.duration}
            className="w-[100px]"
            onChange={(e: ChangeEvent<HTMLInputElement>) => {
              setValue((v) => ({
                ...v,
                duration: +e.target.value.replace(/\D/, ''),
              }))
            }}
          >
            <InputAdornment position="end">s</InputAdornment>
          </TextField>
        </ListItem>

        {!value.pac && (
          <ListItem className="px-0.5 py-1.5">
            <ListItemText
              primary={t(
                'settings.modals.sysproxy.fields.alwaysUseDefaultBypass',
              )}
            />
            <Switch
              disabled={!enabled}
              checked={value.use_default}
              onCheckedChange={(checked) => {
                if (!checked && !value.bypass) {
                  const nextBypass = defaultBypass()
                  setValue((v) => ({
                    ...v,
                    use_default: checked,
                    bypass: nextBypass,
                  }))
                  return
                }
                setValue((v) => ({ ...v, use_default: checked }))
              }}
            />
          </ListItem>
        )}

        {!value.pac && (
          <ListItem className="px-0.5 py-1.5">
            <ListItemText
              primary={t('settings.modals.sysproxy.fields.enableBypassCheck')}
            />
            <Switch
              disabled={!enabled}
              checked={value.enable_bypass_check}
              onCheckedChange={(checked) =>
                setValue((v) => ({ ...v, enable_bypass_check: checked }))
              }
            />
          </ListItem>
        )}

        {!value.pac && !value.use_default && (
          <BaseSplitChipEditor
            value={value.bypass ?? ''}
            separator={separator}
            disabled={!enabled}
            error={bypassError}
            helperText={
              bypassError
                ? t('settings.modals.sysproxy.messages.invalidBypass')
                : undefined
            }
            placeholder="localhost"
            ariaLabel={t('settings.modals.sysproxy.fields.proxyBypass')}
            onChange={(nextValue) => {
              setValue((v) => ({ ...v, bypass: nextValue }))
            }}
            renderHeader={(modeToggle) => (
              <ListItem className="px-0.5 py-1.5">
                <ListItemText
                  primary={t('settings.modals.sysproxy.fields.proxyBypass')}
                />
                {modeToggle ? (
                  <div className="ml-auto">{modeToggle}</div>
                ) : null}
              </ListItem>
            )}
          />
        )}

        {!value.pac && value.use_default && (
          <>
            <ListItemText
              primary={t('settings.modals.sysproxy.fields.bypass')}
            />
            <div className="px-0.5 pb-1.5">
              <div className="flex flex-wrap gap-1">
                {splitBypass(defaultBypass()).map((item) => (
                  <Chip key={item} label={item} size="small" />
                ))}
              </div>
            </div>
          </>
        )}

        {value.pac && (
          <ListItem className="items-start px-0.5 py-1.5">
            <ListItemText
              primary={t('settings.modals.sysproxy.fields.pacScriptContent')}
              className="py-1"
            />
            <Button
              variant="outlined"
              onClick={openPacEditor}
            >
              <Edit className="mr-2 h-4 w-4" />
              {t('settings.modals.sysproxy.actions.editPac')}
            </Button>
            {editorOpen && (
              <EditorViewer
                open={true}
                title={t('settings.modals.sysproxy.actions.editPac')}
                value={pacEditorValue}
                language="javascript"
                path="sysproxy-pac.js"
                dirty={pacEditorValue !== pacEditorSavedValue}
                onChange={setPacEditorValue}
                onSave={handleSavePac}
                onClose={() => setEditorOpen(false)}
              />
            )}
          </ListItem>
        )}
      </List>
    </BaseDialog>
  )
}
