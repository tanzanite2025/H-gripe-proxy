import { useLockFn } from 'ahooks'
import { Copy } from 'lucide-react'
import { useImperativeHandle, useState, type ChangeEvent, type Ref } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog, DialogRef, Switch } from '@/components/base'
import {
  Alert,
  Box,
  IconButton,
  List,
  ListItem,
  ListItemText,
  Snackbar,
  TextField,
  Tooltip,
} from '@/components/tailwind'
import { Spinner } from '@/components/tailwind/icons'
import { useClashInfo } from '@/hooks/data'
import { useVerge } from '@/hooks/system'
import { showNotice } from '@/services/notice-service'

export function ControllerViewer({ ref }: { ref?: Ref<DialogRef> }) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const [copySuccess, setCopySuccess] = useState<null | string>(null)
  const [isSaving, setIsSaving] = useState(false)

  const { clashInfo, patchInfo } = useClashInfo()
  const { verge, patchVerge } = useVerge()
  const [controller, setController] = useState(clashInfo?.server || '')
  const [secret, setSecret] = useState(clashInfo?.secret || '')
  const [enableController, setEnableController] = useState(
    verge?.enable_external_controller ?? false,
  )

  // 对话框打开时初始化配置
  useImperativeHandle(ref, () => ({
    open: async () => {
      setOpen(true)
      setController(clashInfo?.server || '')
      setSecret(clashInfo?.secret || '')
      setEnableController(verge?.enable_external_controller ?? false)
    },
    close: () => setOpen(false),
  }))

  // 保存配置
  const onSave = useLockFn(async () => {
    try {
      setIsSaving(true)

      // 先保存 enable_external_controller 设置
      await patchVerge({ enable_external_controller: enableController })

      // 如果启用了外部控制器，则保存控制器地址和密钥
      if (enableController) {
        if (!controller.trim()) {
          showNotice.error(
            'settings.sections.externalController.messages.addressRequired',
          )
          return
        }

        if (!secret.trim()) {
          showNotice.error(
            'settings.sections.externalController.messages.secretRequired',
          )
          return
        }

        await patchInfo({ 'external-controller': controller, secret })
      } else {
        // 如果禁用了外部控制器，则清空控制器地址
        await patchInfo({ 'external-controller': '' })
      }

      showNotice.success('shared.feedback.notifications.common.saveSuccess')
      setOpen(false)
    } catch (err) {
      showNotice.error(
        'shared.feedback.notifications.common.saveFailed',
        err,
        4000,
      )
    } finally {
      setIsSaving(false)
    }
  })

  // 复制到剪贴板
  const handleCopyToClipboard = useLockFn(
    async (text: string, type: string) => {
      try {
        await navigator.clipboard.writeText(text)
        setCopySuccess(type)
        setTimeout(() => setCopySuccess(null))
      } catch (err) {
        console.warn('[ControllerViewer] copy to clipboard failed:', err)
        showNotice.error(
          'settings.sections.externalController.messages.copyFailed',
        )
      }
    },
  )

  return (
    <BaseDialog
      open={open}
      title={t('settings.sections.externalController.title')}
      panelStyle={{ width: 400 }}
      okBtn={
        isSaving ? (
          <Box className="flex items-center gap-4">
            <Spinner className="h-4 w-4" />
            {t('shared.statuses.saving')}
          </Box>
        ) : (
          t('shared.actions.save')
        )
      }
      cancelBtn={t('shared.actions.cancel')}
      onClose={() => setOpen(false)}
      onCancel={() => setOpen(false)}
      onOk={onSave}
    >
      <List>
        <ListItem className="py-2 px-0 flex justify-between">
          <ListItemText
            primary={t('settings.sections.externalController.fields.enable')}
          />
          <Switch
            checked={enableController}
            onCheckedChange={setEnableController}
            disabled={isSaving}
          />
        </ListItem>

        <ListItem className="py-2 px-0 flex justify-between">
          <ListItemText
            primary={t('settings.sections.externalController.fields.address')}
          />
          <Box className="flex items-center gap-4">
            <TextField
              size="small"
              className={`w-[175px] ${enableController ? 'opacity-100' : 'opacity-50 pointer-events-none'}`}
              value={controller}
              placeholder={t(
                'settings.sections.externalController.placeholders.address',
              )}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setController(e.target.value)}
              disabled={isSaving || !enableController}
            />
            <Tooltip
              title={t('settings.sections.externalController.tooltips.copy')}
            >
              <IconButton
                size="small"
                onClick={() => handleCopyToClipboard(controller, 'controller')}
                className="text-primary"
                disabled={isSaving || !enableController}
              >
                <Copy className="h-4 w-4" />
              </IconButton>
            </Tooltip>
          </Box>
        </ListItem>

        <ListItem className="py-2 px-0 flex justify-between">
          <ListItemText
            primary={t('settings.sections.externalController.fields.secret')}
          />
          <Box className="flex items-center gap-4">
            <TextField
              size="small"
              className={`w-[175px] ${enableController ? 'opacity-100' : 'opacity-50 pointer-events-none'}`}
              value={secret}
              placeholder={t(
                'settings.sections.externalController.placeholders.secret',
              )}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setSecret(e.target.value)}
              disabled={isSaving || !enableController}
            />
            <Tooltip
              title={t('settings.sections.externalController.tooltips.copy')}
            >
              <IconButton
                size="small"
                onClick={() => handleCopyToClipboard(secret, 'secret')}
                className="text-primary"
                disabled={isSaving || !enableController}
              >
                <Copy className="h-4 w-4" />
              </IconButton>
            </Tooltip>
          </Box>
        </ListItem>
      </List>

      <Snackbar
        open={copySuccess !== null}
        autoHideDuration={2000}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
      >
        <Alert severity="success">
          {copySuccess === 'controller'
            ? t(
                'settings.sections.externalController.messages.controllerCopied',
              )
            : t('settings.sections.externalController.messages.secretCopied')}
        </Alert>
      </Snackbar>
    </BaseDialog>
  )
}
