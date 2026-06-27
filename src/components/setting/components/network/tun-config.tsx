import { useLockFn } from 'ahooks'
import type { Ref } from 'react'
import { useImperativeHandle, useState, type ChangeEvent } from 'react'
import { useTranslation } from 'react-i18next'

import {
  BaseDialog,
  BaseSplitChipEditor,
  DialogRef,
  Switch,
} from '@/components/base'
import {
  Box,
  Button,
  List,
  ListItem,
  ListItemText,
  TextField,
  Typography,
} from '@/components/tailwind'
import { useClash } from '@/hooks/data'
import { enhanceProfiles } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { areValidIpCidrs } from '@/utils/network'

import { StackModeSwitch } from '../misc/stack-mode-switch'

const splitRouteExcludeAddress = (value: string) =>
  value
    .split(/[,\n;\r]+/)
    .map((item) => item.trim())
    .filter(Boolean)

export function TunViewer({ ref }: { ref?: Ref<DialogRef> }) {
  const { t } = useTranslation()

  const { clash, mutateClash, patchClash } = useClash()

  const [open, setOpen] = useState(false)
  const [values, setValues] = useState({
    stack: 'mixed',
    device: 'Mihomo',
    autoRoute: true,
    routeExcludeAddress: '',
    autoDetectInterface: true,
    dnsHijack: ['any:53'],
    strictRoute: true,
    mtu: 1500,
  })

  const routeExcludeAddressItems = splitRouteExcludeAddress(
    values.routeExcludeAddress,
  )
  const routeExcludeAddressError =
    values.autoRoute &&
    routeExcludeAddressItems.length > 0 &&
    !areValidIpCidrs(routeExcludeAddressItems)
  const routeExcludeAddressHelperText = routeExcludeAddressError
    ? t('settings.modals.tun.messages.invalidRouteExcludeAddress')
    : t('settings.modals.tun.messages.routeExcludeAddressHint')

  useImperativeHandle(ref, () => ({
    open: () => {
      setOpen(true)
      const nextAutoRoute = clash?.tun['auto-route'] ?? true
      setValues({
        stack: clash?.tun.stack ?? 'gvisor',
        device: clash?.tun.device ?? 'Mihomo',
        autoRoute: nextAutoRoute,
        routeExcludeAddress: (clash?.tun['route-exclude-address'] ?? []).join(
          ',',
        ),
        autoDetectInterface: clash?.tun['auto-detect-interface'] ?? true,
        dnsHijack: clash?.tun['dns-hijack'] ?? ['any:53'],
        strictRoute: clash?.tun['strict-route'] ?? true,
        mtu: clash?.tun.mtu ?? 1500,
      })
    },
    close: () => setOpen(false),
  }))

  const onSave = useLockFn(async () => {
    try {
      const routeExcludeAddress = routeExcludeAddressItems

      if (routeExcludeAddressError) {
        showNotice.error(
          'settings.modals.tun.messages.invalidRouteExcludeAddress',
        )
        return
      }

      const tun: IConfigData['tun'] = {
        stack: values.stack,
        device: values.device === '' ? 'Mihomo' : values.device,
        'auto-route': values.autoRoute,
        'route-exclude-address': routeExcludeAddress,
        'auto-detect-interface': values.autoDetectInterface,
        'dns-hijack': values.dnsHijack[0] === '' ? [] : values.dnsHijack,
        'strict-route': values.strictRoute,
        mtu: values.mtu ?? 1500,
      }
      await patchClash({ tun })
      await mutateClash(
        (old) => ({
          ...old!,
          tun,
        }),
        false,
      )
      setOpen(false)
      showNotice.success('settings.modals.tun.messages.applied')
      void enhanceProfiles().catch((err: any) => {
        showNotice.error(err)
      })
    } catch (err: any) {
      showNotice.error(err)
    }
  })

  return (
    <BaseDialog
      open={open}
      title={
        <Box className="flex justify-between gap-4">
          <Typography variant="h6">{t('settings.modals.tun.title')}</Typography>
          <Button
            variant="outlined"
            size="small"
            onClick={async () => {
              const tun: IConfigData['tun'] = {
                stack: 'gvisor',
                device: 'Mihomo',
                'auto-route': true,
                'auto-detect-interface': true,
                'dns-hijack': ['any:53'],
                'route-exclude-address': [],
                'strict-route': true,
                mtu: 1500,
              }
              setValues({
                stack: 'gvisor',
                device: 'Mihomo',
                autoRoute: true,
                routeExcludeAddress: '',
                autoDetectInterface: true,
                dnsHijack: ['any:53'],
                strictRoute: true,
                mtu: 1500,
              })
              await patchClash({ tun })
              await mutateClash(
                (old) => ({
                  ...old!,
                  tun,
                }),
                false,
              )
            }}
          >
            {t('shared.actions.resetToDefault')}
          </Button>
        </Box>
      }
      panelStyle={{ width: 650 }}
      okBtn={t('shared.actions.save')}
      cancelBtn={t('shared.actions.cancel')}
      onClose={() => setOpen(false)}
      onCancel={() => setOpen(false)}
      onOk={onSave}
    >
      <List>
        <ListItem className="py-[5px] px-[2px]">
          <ListItemText className="flex-none w-[120px] mr-2" primary={t('settings.modals.tun.fields.stack')} />
          <StackModeSwitch
            value={values.stack}
            onChange={(value) => {
              setValues((v) => ({
                ...v,
                stack: value,
              }))
            }}
          />
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText className="flex-none w-[120px] mr-2" primary={t('settings.modals.tun.fields.device')} />
          <TextField
            autoComplete="new-password"
            size="small"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck="false"
            className="w-[300px]"
            value={values.device}
            placeholder="Mihomo"
            onChange={(e: ChangeEvent<HTMLInputElement>) =>
              setValues((v) => ({ ...v, device: e.target.value }))
            }
          />
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText className="flex-none w-[120px] mr-2" primary={t('settings.modals.tun.fields.autoRoute')} />
          <Switch
            checked={values.autoRoute}
            onCheckedChange={(checked) =>
              setValues((v) => ({
                ...v,
                autoRoute: checked,
              }))
            }
          />
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText className="flex-none w-[120px] mr-2" primary={t('settings.modals.tun.fields.strictRoute')} />
          <Switch
            checked={values.strictRoute}
            onCheckedChange={(checked) => setValues((v) => ({ ...v, strictRoute: checked }))}
          />
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            className="flex-none w-[120px] mr-2"
            primary={t('settings.modals.tun.fields.autoDetectInterface')}
          />
          <Switch
            checked={values.autoDetectInterface}
            onCheckedChange={(checked) =>
              setValues((v) => ({ ...v, autoDetectInterface: checked }))
            }
          />
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText className="flex-none w-[120px] mr-2" primary={t('settings.modals.tun.fields.dnsHijack')} />
          <TextField
            autoComplete="new-password"
            size="small"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck="false"
            className="w-[300px]"
            value={values.dnsHijack.join(',')}
            placeholder={t('settings.modals.tun.tooltips.dnsHijack')}
            onChange={(e: ChangeEvent<HTMLInputElement>) =>
              setValues((v) => ({ ...v, dnsHijack: e.target.value.split(',') }))
            }
          />
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText className="flex-none w-[120px] mr-2" primary={t('settings.modals.tun.fields.mtu')} />
          <TextField
            autoComplete="new-password"
            size="small"
            type="number"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck="false"
            className="w-[300px]"
            value={values.mtu}
            placeholder="1500"
            onChange={(e: ChangeEvent<HTMLInputElement>) =>
              setValues((v) => ({
                ...v,
                mtu: parseInt(e.target.value),
              }))
            }
          />
        </ListItem>

        <BaseSplitChipEditor
          value={values.routeExcludeAddress}
          placeholder="192.168.0.0/16"
          ariaLabel={t('settings.modals.tun.fields.routeExcludeAddress')}
          disabled={!values.autoRoute}
          error={routeExcludeAddressError}
          helperText={routeExcludeAddressHelperText}
          onChange={(nextValue) =>
            setValues((v) => ({ ...v, routeExcludeAddress: nextValue }))
          }
          renderHeader={(modeToggle) => (
            <ListItem className="py-[5px] px-[2px]">
              <ListItemText
                className="flex-none w-[120px] mr-2"
                primary={t('settings.modals.tun.fields.routeExcludeAddress')}
              />
              {modeToggle ? (
                <Box className="ml-auto">{modeToggle}</Box>
              ) : null}
            </ListItem>
          )}
        />
      </List>
    </BaseDialog>
  )
}
