import { TextField } from '@/components/tailwind'
import type { ComponentProps } from 'react'
import { useTranslation } from 'react-i18next'

export const BaseStyledTextField = (props: ComponentProps<typeof TextField>) => {
  const { t } = useTranslation()

  return (
    <TextField
      placeholder={t('shared.placeholders.filter')}
      className="bg-white dark:bg-transparent"
      {...props}
    />
  )
}
