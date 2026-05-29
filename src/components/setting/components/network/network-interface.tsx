import { Copy } from 'lucide-react'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'
import type { Ref } from 'react'
import { useImperativeHandle, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog, DialogRef } from '@/components/base'
import { Box, Button, IconButton } from '@/components/tailwind'
import { useNetworkInterfaces } from '@/hooks/network/use-network'
import { showNotice } from '@/services/notice-service'

export function NetworkInterfaceViewer({ ref }: { ref?: Ref<DialogRef> }) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const [isV4, setIsV4] = useState(true)

  useImperativeHandle(ref, () => ({
    open: () => {
      setOpen(true)
    },
    close: () => setOpen(false),
  }))

  const { networkInterfaces } = useNetworkInterfaces()

  return (
    <BaseDialog
      open={open}
      title={
        <Box className="flex justify-between">
          {t('settings.modals.networkInterface.title')}
          <Box>
            <Button
              variant="contained"
              size="small"
              onClick={() => {
                setIsV4((prev) => !prev)
              }}
            >
              {isV4 ? 'Ipv6' : 'Ipv4'}
            </Button>
          </Box>
        </Box>
      }
      panelStyle={{ width: 450 }}
      disableOk
      cancelBtn={t('shared.actions.close')}
      onClose={() => setOpen(false)}
      onCancel={() => setOpen(false)}
    >
      {networkInterfaces.map((item) => (
        <Box key={item.name}>
          <h4>{item.name}</h4>
          <Box>
            {isV4 && (
              <>
                {item.addr.map(
                  (address) =>
                    address.V4 && (
                      <AddressDisplay
                        key={address.V4.ip}
                        label={t(
                          'settings.modals.networkInterface.fields.ipAddress',
                        )}
                        content={address.V4.ip}
                      />
                    ),
                )}
                <AddressDisplay
                  label={t(
                    'settings.modals.networkInterface.fields.macAddress',
                  )}
                  content={item.mac_addr ?? ''}
                />
              </>
            )}
            {!isV4 && (
              <>
                {item.addr.map(
                  (address) =>
                    address.V6 && (
                      <AddressDisplay
                        key={address.V6.ip}
                        label={t(
                          'settings.modals.networkInterface.fields.ipAddress',
                        )}
                        content={address.V6.ip}
                      />
                    ),
                )}
                <AddressDisplay
                  label={t(
                    'settings.modals.networkInterface.fields.macAddress',
                  )}
                  content={item.mac_addr ?? ''}
                />
              </>
            )}
          </Box>
        </Box>
      ))}
    </BaseDialog>
  )
}

const AddressDisplay = ({
  label,
  content,
}: {
  label: string
  content: string
}) => {
  return (
    <Box className="flex justify-between my-2">
      <Box>{label}</Box>
      <Box className="inline-flex items-center rounded-lg bg-black/5 py-[2px] pl-2 pr-[2px] dark:bg-white/10">
        <Box className="inline select-text">{content}</Box>
        <IconButton
          size="small"
          onClick={async () => {
            await writeText(content)
            showNotice.success(
              'shared.feedback.notifications.common.copySuccess',
            )
          }}
        >
          <Copy className="text-[18px]" />
        </IconButton>
      </Box>
    </Box>
  )
}
