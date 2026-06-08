import dayjs from 'dayjs'
import { Share2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { CircularProgress } from '@/components/tailwind/CircularProgress'
import { IconButton } from '@/components/tailwind/IconButton'
import { LinearProgress } from '@/components/tailwind/LinearProgress'
import parseTraffic from '@/utils/format'

import { ProfileBox } from './profile-box'
import { ProfileCardActions } from './profile-card-actions'
import { formatExpireDate } from './profile-item/shared'

interface ProfileItemUIProps {
  // Basic info
  name: string
  description?: string
  from?: string
  hasUrl: boolean
  hasExtra: boolean
  hasHome: boolean

  // State
  selected: boolean
  activating: boolean
  loading: boolean
  isDragging: boolean
  batchMode?: boolean
  isSelected?: boolean

  // Time display
  updated: number
  showNextUpdate: boolean
  nextUpdateTime: string

  // Traffic info
  upload: number
  download: number
  total: number
  expire?: string
  progress: number

  // Drag and drop
  dragHandleProps: {
    ref: (node: HTMLElement | null) => void
    attributes: any
    listeners: any
  }
  transform?: { x: number; y: number; scaleX: number; scaleY: number } | null
  transition?: string

  // Event handlers
  onClick: (e: React.MouseEvent) => void
  onContextMenu: (e: React.MouseEvent) => void
  onUseClick: (e: React.MouseEvent<HTMLButtonElement>) => void
  onDirectUpdateClick: (e: React.MouseEvent<HTMLButtonElement>) => void
  onProxyUpdateClick: (e: React.MouseEvent<HTMLButtonElement>) => void
  onEditProxiesClick: (e: React.MouseEvent<HTMLButtonElement>) => void
  onEditGroupsClick: (e: React.MouseEvent<HTMLButtonElement>) => void
  onShareQrCodeClick: (e: React.MouseEvent<HTMLButtonElement>) => void
  canEditProxies: boolean
  canEditGroups: boolean
  onToggleUpdateTimeDisplay: (e: React.MouseEvent) => void
  onSelectionChange?: () => void
}

export const ProfileItemUI = (props: ProfileItemUIProps) => {
  const {
    name,
    description,
    from,
    hasUrl,
    hasExtra,
    selected,
    activating,
    loading,
    isDragging,
    batchMode,
    isSelected,
    updated,
    showNextUpdate,
    nextUpdateTime,
    upload,
    download,
    total,
    expire,
    progress,
    dragHandleProps,
    transform,
    transition,
    onClick,
    onContextMenu,
    onUseClick,
    onDirectUpdateClick,
    onProxyUpdateClick,
    onEditProxiesClick,
    onEditGroupsClick,
    onShareQrCodeClick,
    canEditProxies,
    canEditGroups,
    onToggleUpdateTimeDisplay,
    onSelectionChange,
  } = props

  const { t } = useTranslation()

  const transformStyle = transform
    ? {
        transform: `translate3d(${transform.x}px, ${transform.y}px, 0) scaleX(${transform.scaleX}) scaleY(${transform.scaleY})`,
      }
    : undefined

  return (
    <div
      className="relative"
      style={{
        ...transformStyle,
        transition,
        zIndex: isDragging ? 9999 : undefined,
      }}
    >
      <ProfileBox
        aria-selected={selected}
        onClick={onClick}
        onContextMenu={onContextMenu}
      >
        {activating && (
          <div className="absolute top-2.5 left-2.5 right-2.5 bottom-0.5 z-10 flex items-center justify-center backdrop-blur-sm bg-black/10">
            <CircularProgress size={20} className="animate-pulse" />
          </div>
        )}

        <div className="relative">
          <div className="flex items-start gap-1">
            {batchMode && (
              <IconButton
                size="small"
                className="-ml-2 mr-1 p-0.5"
                onClick={(e) => {
                  e.stopPropagation()
                  if (onSelectionChange) {
                    onSelectionChange()
                  }
                }}
              >
                {isSelected ? (
                  <svg
                    className="w-6 h-6 text-primary"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path d="M19 3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.11 0 2-.9 2-2V5c0-1.1-.89-2-2-2zm-9 14l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z" />
                  </svg>
                ) : (
                  <svg
                    className="w-6 h-6"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path d="M19 5v14H5V5h14m0-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2z" />
                  </svg>
                )}
              </IconButton>
            )}

            <div
              ref={dragHandleProps.ref}
              className={`my-auto flex shrink-0 ${batchMode ? '-ml-1' : ''}`}
              {...dragHandleProps.attributes}
              {...dragHandleProps.listeners}
            >
              <svg
                className="w-6 h-6 -ml-1.5 cursor-move text-current"
                viewBox="0 0 24 24"
                fill="currentColor"
              >
                <path d="M11 18c0 1.1-.9 2-2 2s-2-.9-2-2 .9-2 2-2 2 .9 2 2zm-2-8c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm0-6c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm6 4c1.1 0 2-.9 2-2s-.9-2-2-2-2 .9-2 2 .9 2 2 2zm0 2c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm0 6c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2z" />
              </svg>
            </div>

            <h2
              className="min-w-0 flex-1 truncate text-lg font-semibold leading-[26px]"
              title={name}
            >
              {name}
            </h2>

            {hasUrl && (
              <IconButton
                size="small"
                color="inherit"
                className="-mr-1 mt-[-2px] h-7 w-7 shrink-0 text-text-secondary hover:bg-white/10 hover:text-text-primary"
                title={t('profiles.components.menu.shareQrCode')}
                aria-label={t('profiles.components.menu.shareQrCode')}
                onClick={(event) => {
                  event.stopPropagation()
                  onShareQrCodeClick(event)
                }}
              >
                <Share2 className="h-4 w-4" />
              </IconButton>
            )}
          </div>
        </div>

        {/* Second line: description or URL */}
        <div className="h-[26px] flex items-center justify-between">
          {description ? (
            <p className="text-sm truncate" title={description}>
              {description}
            </p>
          ) : (
            hasUrl && (
              <p className="truncate" title={`${t('shared.labels.from')} ${from}`}>
                {from}
              </p>
            )
          )}
          {hasUrl && (
            <div className="flex justify-end ml-auto">
              <span
                className="text-sm text-right cursor-pointer inline-block border-b border-transparent transition-all duration-200 hover:border-primary hover:text-primary"
                title={
                  showNextUpdate
                    ? t('profiles.components.profileItem.tooltips.showLast')
                    : `${t('shared.labels.updateTime')}: ${formatExpireDate(updated)}\n${t('profiles.components.profileItem.tooltips.showNext')}`
                }
                onClick={onToggleUpdateTimeDisplay}
              >
                {showNextUpdate
                  ? nextUpdateTime
                  : updated > 0
                    ? dayjs(updated * 1000).fromNow()
                    : ''}
              </span>
            </div>
          )}
        </div>

        {/* Third line: traffic info or update time */}
        {hasExtra ? (
          <div className="h-[26px] flex items-center justify-between text-sm">
            <span title={t('shared.labels.usedTotal')}>
              {parseTraffic(upload + download)} / {parseTraffic(total)}
            </span>
            <span title={t('shared.labels.expireTime')}>{expire}</span>
          </div>
        ) : (
          <div className="h-[26px] flex items-center justify-end text-xs">
            <span title={t('shared.labels.updateTime')}>
              {formatExpireDate(updated)}
            </span>
          </div>
        )}

        <LinearProgress
          variant="determinate"
          value={progress}
          style={{ opacity: total > 0 ? 1 : 0 }}
        />

        {!batchMode && (
          <ProfileCardActions
            hasUrl={hasUrl}
            selected={selected}
            activating={activating}
            loading={loading}
            canEditProxies={canEditProxies}
            canEditGroups={canEditGroups}
            onUseClick={onUseClick}
            onDirectUpdateClick={onDirectUpdateClick}
            onProxyUpdateClick={onProxyUpdateClick}
            onEditProxiesClick={onEditProxiesClick}
            onEditGroupsClick={onEditGroupsClick}
          />
        )}
      </ProfileBox>
    </div>
  )
}
