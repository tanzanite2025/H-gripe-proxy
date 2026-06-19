import { useTranslation } from 'react-i18next'

import { Button } from '@/components/tailwind/Button'
import {
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
} from '@/components/tailwind/Dialog'
import { List } from '@/components/tailwind/List'
import type { RuntimeProviderHealthRecord } from '@/services/proxy-runtime'
import type { IProxyProviderItem } from '@/types/proxy'

import { ProviderListItem } from './provider-list-item'

interface ProviderDialogProps {
  open: boolean
  providers: Array<[string, IProxyProviderItem]>
  updating: Record<string, boolean>
  checking: Record<string, boolean>
  health: Record<string, RuntimeProviderHealthRecord>
  onClose: () => void
  onUpdateAll: () => void | Promise<void>
  onUpdateProvider: (name: string) => void | Promise<void>
  onCheckProvider: (name: string) => void | Promise<void>
}

export const ProviderDialog = ({
  open,
  providers,
  updating,
  checking,
  health,
  onClose,
  onUpdateAll,
  onUpdateProvider,
  onCheckProvider,
}: ProviderDialogProps) => {
  const { t } = useTranslation()

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth="sm"
      fullWidth
      className="uds-dialog"
    >
      <DialogTitle className="uds-title-h2">
        <div className="flex items-center justify-between">
          <h6 className="uds-title-h2 text-lg font-semibold">
            {t('proxies.page.provider.title')}
          </h6>
          <Button
            variant="contained"
            size="small"
            onClick={onUpdateAll}
            aria-label={t('proxies.page.provider.actions.updateAll')}
          >
            {t('proxies.page.provider.actions.updateAll')}
          </Button>
        </div>
      </DialogTitle>

      <DialogContent>
        <List className="min-h-[250px] py-0">
          {providers.map(([name, provider]) => (
            <ProviderListItem
              key={name}
              name={name}
              provider={provider}
              isUpdating={!!updating[name]}
              isChecking={!!checking[name]}
              health={health[name]}
              onUpdate={onUpdateProvider}
              onCheck={onCheckProvider}
            />
          ))}
        </List>
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose} variant="outlined">
          {t('shared.actions.close')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
