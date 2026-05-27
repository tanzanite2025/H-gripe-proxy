import { Fragment } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseEmpty } from '@/components/base'
import { Button } from '@/components/tailwind/Button'
import {
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
} from '@/components/tailwind/Dialog'

interface Props {
  open: boolean
  logInfo: [string, string][]
  onClose: () => void
}

export const LogViewer = (props: Props) => {
  const { open, logInfo, onClose } = props

  const { t } = useTranslation()

  return (
    <Dialog open={open} onClose={onClose}>
      <DialogTitle>{t('profiles.modals.logViewer.title')}</DialogTitle>

      <DialogContent className="w-[400px] h-[300px] overflow-x-hidden select-text pb-2">
        {logInfo.map(([level, log]) => (
          <Fragment key={`${level}-${log}`}>
            <div className="text-gray-400 dark:text-gray-500">
              <span
                className={`inline-block px-2 py-0.5 mr-2 text-xs border rounded ${
                  level === 'error' || level === 'exception'
                    ? 'border-red-500 text-red-500'
                    : 'border-gray-400 dark:border-gray-600 text-gray-600 dark:text-gray-400'
                }`}
              >
                {level}
              </span>
              {log}
            </div>
            <div className="border-t border-gray-200 dark:border-gray-700 my-1" />
          </Fragment>
        ))}

        {logInfo.length === 0 && <BaseEmpty />}
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose} variant="outlined">
          {t('shared.actions.close')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
