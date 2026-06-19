import { Database } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/tailwind/Button'

import { ProviderDialog } from './provider-button/provider-dialog'
import { useProviderButtonController } from './provider-button/use-provider-button-controller'

export const ProviderButton = () => {
  const { t } = useTranslation()
  const controller = useProviderButtonController()

  if (!controller.hasProviders) return null

  return (
    <>
      <Button
        variant="outlined"
        size="small"
        startIcon={<Database className="h-4 w-4" />}
        onClick={controller.openDialog}
        className="mr-2"
      >
        {t('proxies.page.provider.title')}
      </Button>

      <ProviderDialog
        open={controller.open}
        providers={controller.providers}
        updating={controller.updating}
        checking={controller.checking}
        health={controller.health}
        onClose={controller.closeDialog}
        onUpdateAll={controller.updateAllProviders}
        onUpdateProvider={controller.updateProvider}
        onCheckProvider={controller.checkProvider}
      />
    </>
  )
}
