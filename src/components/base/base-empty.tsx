import { Inbox } from 'lucide-react'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'

import type { TranslationKey } from '@/types/generated/i18n-keys'

interface Props {
  text?: ReactNode
  textKey?: TranslationKey
  extra?: ReactNode
}

export const BaseEmpty = ({
  text,
  textKey = 'shared.statuses.empty',
  extra,
}: Props) => {
  const { t } = useTranslation()

  const resolvedText: ReactNode = text !== undefined ? text : t(textKey)

  return (
    <div className="flex h-full w-full flex-col items-center justify-center text-gray-500/75 dark:text-gray-400/75">
      <Inbox className="h-16 w-16" />
      <p className="text-xl">{resolvedText}</p>
      {extra}
    </div>
  )
}
