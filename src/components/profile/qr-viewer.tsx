import { QRCodeSVG } from 'qrcode.react'
import { useTranslation } from 'react-i18next'

import { Dialog, DialogContent, DialogTitle } from '@/components/tailwind'

interface Props {
  open: boolean
  value: string
  title?: string
  onClose: () => void
}

export const QrViewer = (props: Props) => {
  const { open, value, title, onClose } = props
  const { t } = useTranslation()

  return (
    <Dialog open={open} onClose={onClose} maxWidth="xs">
      <DialogTitle>{title ?? t('profiles.modals.qrViewer.title')}</DialogTitle>
      <DialogContent className="pb-6">
        <div className="flex justify-center rounded-lg bg-white p-4">
          <QRCodeSVG value={value} size={256} level="M" />
        </div>
      </DialogContent>
    </Dialog>
  )
}
