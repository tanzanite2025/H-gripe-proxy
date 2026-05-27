import { X } from 'lucide-react'
import React, { useCallback, useMemo, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'

import { Snackbar, Alert, IconButton } from '@/components/tailwind'
import {
  subscribeNotices,
  hideNotice,
  getSnapshotNotices,
  showNotice,
} from '@/services/notice-service'
import type { TranslationKey } from '@/types/generated/i18n-keys'

type NoticePosition = NonNullable<IVergeConfig['notice_position']>
type NoticeItem = ReturnType<typeof getSnapshotNotices>[number]
type TranslationFn = ReturnType<typeof useTranslation>['t']

type SnackbarOrigin = {
  vertical: 'top' | 'bottom'
  horizontal: 'left' | 'right' | 'center'
}

const VALID_POSITIONS: NoticePosition[] = [
  'top-left',
  'top-right',
  'bottom-left',
  'bottom-right',
]

const resolvePosition = (position?: NoticePosition | null): NoticePosition => {
  if (position && VALID_POSITIONS.includes(position)) {
    return position
  }
  return 'top-right'
}

const getAnchorOrigin = (position: NoticePosition): SnackbarOrigin => {
  const [vertical, horizontal] = position.split('-') as [
    SnackbarOrigin['vertical'],
    SnackbarOrigin['horizontal'],
  ]
  return { vertical, horizontal }
}

const resolveNoticeMessage = (
  notice: NoticeItem,
  t: TranslationFn,
): React.ReactNode => {
  const i18n = notice.i18n
  if (!i18n) return notice.message

  const params = (i18n.params ?? {}) as Record<string, unknown>
  const { prefixKey, prefixParams, prefix, message, ...restParams } = params

  const prefixKeyParams =
    prefixParams && typeof prefixParams === 'object'
      ? (prefixParams as Record<string, unknown>)
      : undefined

  const resolvedPrefix =
    typeof prefixKey === 'string'
      ? t(prefixKey as TranslationKey, {
          defaultValue: prefixKey,
          ...(prefixKeyParams ?? {}),
          ...restParams,
        })
      : typeof prefix === 'string'
        ? prefix
        : undefined

  const messageStr = typeof message === 'string' ? message : undefined

  const defaultValue =
    messageStr === undefined
      ? undefined
      : resolvedPrefix
        ? `${resolvedPrefix} ${messageStr}`
        : messageStr

  return t(i18n.key as TranslationKey, {
    defaultValue,
    ...restParams,
    ...(resolvedPrefix !== undefined ? { prefix: resolvedPrefix } : {}),
    ...(messageStr !== undefined ? { message: messageStr } : {}),
  })
}

const extractNoticeCopyText = (input: unknown): string | undefined => {
  if (input === null || input === undefined) return undefined
  if (typeof input === 'string') return input
  if (typeof input === 'number' || typeof input === 'boolean') {
    return String(input)
  }
  if (input instanceof Error) {
    return input.message || input.name
  }
  if (React.isValidElement(input)) return undefined
  if (typeof input === 'object') {
    const maybeMessage = (input as { message?: unknown }).message
    if (typeof maybeMessage === 'string') return maybeMessage
  }
  try {
    return JSON.stringify(input)
  } catch {
    return String(input)
  }
}

const resolveNoticeCopyText = (
  notice: NoticeItem,
  t: TranslationFn,
): string | undefined => {
  if (
    notice.i18n?.key === 'shared.feedback.notices.prefixedRaw' ||
    notice.i18n?.key === 'shared.feedback.notices.raw'
  ) {
    const rawText = extractNoticeCopyText(notice.i18n?.params?.message)
    if (rawText) return rawText
  }

  return (
    extractNoticeCopyText(resolveNoticeMessage(notice, t)) ??
    extractNoticeCopyText(notice.message)
  )
}

interface NoticeManagerProps {
  position?: NoticePosition | null
}

export const NoticeManager: React.FC<NoticeManagerProps> = ({ position }) => {
  const { t } = useTranslation()
  const resolvedPosition = useMemo(() => resolvePosition(position), [position])
  const anchorOrigin = useMemo(
    () => getAnchorOrigin(resolvedPosition),
    [resolvedPosition],
  )
  const currentNotices = useSyncExternalStore(
    subscribeNotices,
    getSnapshotNotices,
  )

  const handleClose = (id: number) => {
    hideNotice(id)
  }

  const handleNoticeCopy = useCallback(
    async (notice: NoticeItem) => {
      const text = resolveNoticeCopyText(notice, t)
      if (!text) return
      try {
        await navigator.clipboard.writeText(text)
        showNotice.success(
          'shared.feedback.notifications.common.copySuccess',
          1000,
        )
      } catch (error) {
        console.warn('[NoticeManager] copy to clipboard failed:', error)
      }
    },
    [t],
  )

  return (
    <div
      className="fixed z-[1500] flex max-w-[360px] flex-col gap-2.5"
      style={{
        top: anchorOrigin.vertical === 'top' ? '20px' : 'auto',
        bottom: anchorOrigin.vertical === 'bottom' ? '20px' : 'auto',
        left: anchorOrigin.horizontal === 'left' ? '20px' : 'auto',
        right: anchorOrigin.horizontal === 'right' ? '20px' : 'auto',
      }}
    >
      {currentNotices.map((notice) => (
        <Snackbar
          key={notice.id}
          open={true}
          anchorOrigin={anchorOrigin}
          className="relative w-full"
          style={{
            transform: 'none',
            top: 'auto',
            right: 'auto',
            bottom: 'auto',
            left: 'auto',
          }}
        >
          <Alert
            severity={notice.type}
            variant="filled"
            className="w-full"
            onContextMenu={(event) => {
              event.preventDefault()
              event.stopPropagation()
              void handleNoticeCopy(notice)
            }}
            action={
              <IconButton
                size="small"
                className="text-inherit"
                onClick={() => handleClose(notice.id)}
              >
                <X className="h-4 w-4" />
              </IconButton>
            }
          >
            {resolveNoticeMessage(notice, t)}
          </Alert>
        </Snackbar>
      ))}
    </div>
  )
}
