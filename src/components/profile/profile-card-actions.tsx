import type { MouseEventHandler } from 'react'
import { useTranslation } from 'react-i18next'

import { cn } from '@/utils/cn'

interface ProfileCardActionsProps {
  hasUrl: boolean
  selected: boolean
  activating: boolean
  loading: boolean
  canEditProxies: boolean
  canEditGroups: boolean
  onUseClick: MouseEventHandler<HTMLButtonElement>
  onDirectUpdateClick: MouseEventHandler<HTMLButtonElement>
  onProxyUpdateClick: MouseEventHandler<HTMLButtonElement>
  onEditProxiesClick: MouseEventHandler<HTMLButtonElement>
  onEditGroupsClick: MouseEventHandler<HTMLButtonElement>
}

const actionButtonClass =
  'h-7 rounded-full border border-solid px-3 text-xs font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-45'

export const ProfileCardActions = ({
  hasUrl,
  selected,
  activating,
  loading,
  canEditProxies,
  canEditGroups,
  onUseClick,
  onDirectUpdateClick,
  onProxyUpdateClick,
  onEditProxiesClick,
  onEditGroupsClick,
}: ProfileCardActionsProps) => {
  const { t } = useTranslation()
  const updatesDisabled = activating || loading
  const handleAction =
    (handler: MouseEventHandler<HTMLButtonElement>): MouseEventHandler<HTMLButtonElement> =>
    (event) => {
      event.stopPropagation()
      handler(event)
    }

  return (
    <div className="mt-3 flex flex-wrap gap-1.5">
      <button
        type="button"
        className={cn(
          actionButtonClass,
          'border-teal-500/45 bg-teal-500/10 text-teal-500 hover:bg-teal-500/15',
        )}
        disabled={activating || selected}
        onClick={handleAction(onUseClick)}
      >
        {t('profiles.components.menu.select')}
      </button>

      {hasUrl && (
        <>
          <button
            type="button"
            className={cn(
              actionButtonClass,
              'border-border bg-white/5 text-text-secondary hover:bg-white/10 hover:text-text-primary',
            )}
            disabled={updatesDisabled}
            onClick={handleAction(onDirectUpdateClick)}
          >
            {t('profiles.components.menu.update')}
          </button>
          <button
            type="button"
            className={cn(
              actionButtonClass,
              'border-border bg-white/5 text-text-secondary hover:bg-white/10 hover:text-text-primary',
            )}
            disabled={updatesDisabled}
            onClick={handleAction(onProxyUpdateClick)}
          >
            {t('profiles.components.menu.updateViaProxy')}
          </button>
        </>
      )}

      <button
        type="button"
        className={cn(
          actionButtonClass,
          'border-border bg-white/5 text-text-secondary hover:bg-white/10 hover:text-text-primary',
        )}
        disabled={!canEditProxies}
        onClick={handleAction(onEditProxiesClick)}
      >
        {t('profiles.components.menu.editProxies')}
      </button>
      <button
        type="button"
        className={cn(
          actionButtonClass,
          'border-border bg-white/5 text-text-secondary hover:bg-white/10 hover:text-text-primary',
        )}
        disabled={!canEditGroups}
        onClick={handleAction(onEditGroupsClick)}
      >
        {t('profiles.components.menu.editGroups')}
      </button>
    </div>
  )
}
