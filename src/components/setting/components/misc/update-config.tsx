import type { DownloadEvent } from '@tauri-apps/plugin-updater'
import { useLockFn } from 'ahooks'
import type { Ref } from 'react'
import { useImperativeHandle, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import ReactMarkdown from 'react-markdown'

import { BaseDialog, DialogRef } from '@/components/base'
import { Box, Button } from '@/components/tailwind'
import { LinearProgress } from '@/components/tailwind/LinearProgress'
import { useUpdate } from '@/hooks/system'
import { portableFlag } from '@/pages/_layout/layout'
import { openWebUrl, restartApp } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { useSetUpdateState, useUpdateState } from '@/services/states'
import { cn } from '@/utils/cn'

type MarkdownNode = {
  type: string
  value?: string
  children?: MarkdownNode[]
  data?: {
    hProperties?: Record<string, unknown>
  }
}

const GITHUB_ALERTS = {
  note: { label: 'Note', color: '#0969da' },
  tip: { label: 'Tip', color: '#1a7f37' },
  important: { label: 'Important', color: '#8250df' },
  warning: { label: 'Warning', color: '#9a6700' },
  caution: { label: 'Caution', color: '#cf222e' },
} as const

type GitHubAlertType = keyof typeof GITHUB_ALERTS

const GITHUB_ALERT_PATTERN =
  /^\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\][\t ]*\n?/i
const GITHUB_ALERT_CLASS_PATTERN =
  /markdown-alert-(note|tip|important|warning|caution)/

const toRgba = (hex: string, alpha: number) => {
  const normalized = hex.replace('#', '')
  const value = Number.parseInt(normalized, 16)

  const red = (value >> 16) & 255
  const green = (value >> 8) & 255
  const blue = value & 255

  return `rgba(${red}, ${green}, ${blue}, ${alpha})`
}

const getAlertTypeFromClassName = (
  className: unknown,
): GitHubAlertType | null => {
  const value = Array.isArray(className)
    ? className.join(' ')
    : typeof className === 'string'
      ? className
      : ''
  const match = value.match(GITHUB_ALERT_CLASS_PATTERN)
  return match?.[1] as GitHubAlertType | null
}

const findFirstTextNode = (node: MarkdownNode): MarkdownNode | null => {
  if (node.type === 'text') return node
  for (const child of node.children ?? []) {
    const result = findFirstTextNode(child)
    if (result) return result
  }
  return null
}

const remarkGitHubAlerts = () => {
  const visit = (node: MarkdownNode) => {
    for (const child of node.children ?? []) {
      visit(child)
    }

    if (node.type !== 'blockquote') return

    const firstTextNode = findFirstTextNode(node)
    const match = firstTextNode?.value?.match(GITHUB_ALERT_PATTERN)
    if (!firstTextNode?.value || !match) return

    const alertType = match[1].toLowerCase() as GitHubAlertType
    firstTextNode.value = firstTextNode.value
      .replace(GITHUB_ALERT_PATTERN, '')
      .replace(/^\n+/, '')

    node.data = {
      ...(node.data ?? {}),
      hProperties: {
        ...(node.data?.hProperties ?? {}),
        className: ['markdown-alert', `markdown-alert-${alertType}`],
      },
    }

    node.children?.unshift({
      type: 'paragraph',
      data: {
        hProperties: {
          className: 'markdown-alert-title',
        },
      },
      children: [
        {
          type: 'text',
          value: GITHUB_ALERTS[alertType].label,
        },
      ],
    })
  }

  return visit
}

export function UpdateViewer({ ref }: { ref?: Ref<DialogRef> }) {
  const { t } = useTranslation()

  const [open, setOpen] = useState(false)
  const updateState = useUpdateState()
  const setUpdateState = useSetUpdateState()

  const { updateInfo } = useUpdate()

  const [downloaded, setDownloaded] = useState(0)
  const [total, setTotal] = useState(0)
  const downloadedRef = useRef(0)
  const totalRef = useRef(0)

  const progress = useMemo(() => {
    if (total <= 0) return 0
    return Math.min((downloaded / total) * 100, 100)
  }, [downloaded, total])

  useImperativeHandle(ref, () => ({
    open: () => setOpen(true),
    close: () => setOpen(false),
  }))

  const markdownContent = useMemo(() => {
    if (!updateInfo?.body) {
      return 'New Version is available'
    }
    return updateInfo?.body
  }, [updateInfo])

  const breakChangeFlag = useMemo(() => {
    if (!updateInfo?.body) {
      return false
    }
    return updateInfo?.body.toLowerCase().includes('break change')
  }, [updateInfo])

  const onUpdate = useLockFn(async () => {
    if (portableFlag) {
      showNotice.error('settings.modals.update.messages.portableError')
      return
    }
    if (!updateInfo?.body) return
    if (breakChangeFlag) {
      showNotice.error('settings.modals.update.messages.breakChangeError')
      return
    }
    if (updateState) return
    setUpdateState(true)
    setDownloaded(0)
    setTotal(0)
    downloadedRef.current = 0
    totalRef.current = 0

    const onDownloadEvent = (event: DownloadEvent) => {
      if (event.event === 'Started') {
        const contentLength = event.data.contentLength ?? 0
        totalRef.current = contentLength
        setTotal(contentLength)
        setDownloaded(0)
        downloadedRef.current = 0
        return
      }

      if (event.event === 'Progress') {
        setDownloaded((prev) => {
          const next = prev + event.data.chunkLength
          downloadedRef.current = next
          return next
        })
      }

      if (event.event === 'Finished' && totalRef.current === 0) {
        totalRef.current = downloadedRef.current
        setTotal(downloadedRef.current)
      }
    }

    try {
      await updateInfo.downloadAndInstall(onDownloadEvent)
      await restartApp()
    } catch (err: any) {
      showNotice.error(err)
    } finally {
      setUpdateState(false)
      setDownloaded(0)
      setTotal(0)
      downloadedRef.current = 0
      totalRef.current = 0
    }
  })

  return (
    <BaseDialog
      open={open}
      title={
        <Box className="flex items-center justify-between gap-8 min-w-0">
          <Box className="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">
            {t('settings.modals.update.title', {
              version: updateInfo?.version ?? '',
            })}
          </Box>
          <Button
            variant="contained"
            size="small"
            className="whitespace-nowrap"
            onClick={() => {
              void openWebUrl(
                `https://github.com/tanzanite2025/clash-verge-optimized/releases/tag/v${updateInfo?.version}`,
              )
            }}
          >
            {t('settings.modals.update.actions.goToRelease')}
          </Button>
        </Box>
      }
      panelClassName="flex flex-col"
      panelStyle={{
        width: 'min(560px, calc(100vw - 56px))',
        maxWidth: 'calc(100vw - 56px)',
        height: 'min(64vh, 680px)',
      }}
      contentClassName="flex min-h-0 flex-1 flex-col pb-1"
      okBtn={t('settings.modals.update.actions.update')}
      cancelBtn={t('shared.actions.cancel')}
      onClose={() => setOpen(false)}
      onCancel={() => setOpen(false)}
      onOk={onUpdate}
    >
      <Box
        className="flex-1 min-h-0 overflow-auto break-words pr-1.5 -mr-1 text-sm leading-[1.65] text-gray-900 dark:text-gray-100 [&>*:first-child]:mt-0 [&>*:last-child]:mb-0 [&_h1]:mt-0 [&_h1]:mb-6 [&_h1]:text-2xl [&_h1]:leading-[1.25] [&_h2]:mt-9 [&_h2]:mb-4 [&_h2]:text-[19px] [&_h2]:leading-[1.3] [&_h3]:mt-8 [&_h3]:mb-3 [&_h3]:text-base [&_h3]:leading-[1.35] [&_h4]:mt-6 [&_h4]:mb-3 [&_h4]:text-sm [&_h4]:leading-[1.4] [&_h5]:mt-6 [&_h5]:mb-3 [&_h5]:text-sm [&_h5]:leading-[1.4] [&_h6]:mt-6 [&_h6]:mb-3 [&_h6]:text-sm [&_h6]:leading-[1.4] [&_p]:my-4 [&_ul]:my-4 [&_ul]:list-disc [&_ul]:pl-11 [&_ol]:my-4 [&_ol]:list-decimal [&_ol]:pl-11 [&_li]:my-[0.35rem] [&_li]:pl-1 [&_a]:break-words [&_a]:text-primary [&_strong]:font-bold [&_code]:rounded-md [&_code]:bg-black/5 [&_code]:px-2 [&_code]:py-0.5 [&_code]:text-[0.92em] dark:[&_code]:bg-white/10 [&_pre]:my-6 [&_pre]:overflow-auto [&_pre]:rounded-xl [&_pre]:bg-black/5 [&_pre]:p-4 dark:[&_pre]:bg-white/10 [&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_pre_code]:text-[0.9em] [&_table]:my-6 [&_table]:block [&_table]:w-full [&_table]:overflow-x-auto [&_table]:border-collapse [&_th]:border [&_th]:border-divider [&_th]:bg-black/5 [&_th]:px-4 [&_th]:py-3 [&_th]:align-top [&_th]:font-bold dark:[&_th]:bg-white/10 [&_td]:border [&_td]:border-divider [&_td]:px-4 [&_td]:py-3 [&_td]:align-top [&_hr]:my-8 [&_hr]:border-0 [&_hr]:border-t [&_hr]:border-divider [&_img]:h-auto [&_img]:max-w-full [&_img]:rounded-lg"
      >
        <ReactMarkdown
          remarkPlugins={[remarkGitHubAlerts]}
          skipHtml
          components={{
            a: ({ ...props }) => {
              const { children } = props
              return (
                <a {...props} target="_blank" rel="noreferrer">
                  {children}
                </a>
              )
            },
            blockquote: ({ className, children }) => {
              const alertType = getAlertTypeFromClassName(className)

              if (!alertType) {
                return (
                  <blockquote className={cn('my-3 border-l-4 border-divider pl-4 text-gray-600 dark:text-gray-400', className)}>
                    {children}
                  </blockquote>
                )
              }

              const color = GITHUB_ALERTS[alertType].color

              return (
                <blockquote
                  className={cn(
                    'my-3 rounded-lg px-4 py-2 [&_p]:my-3 [&_.markdown-alert-title]:flex [&_.markdown-alert-title]:items-center [&_.markdown-alert-title]:gap-3 [&_.markdown-alert-title]:font-bold [&_.markdown-alert-title]:leading-[1.4]',
                    className,
                  )}
                  style={{
                    borderLeft: `4px solid ${color}`,
                    backgroundColor: toRgba(color, 0.12),
                  }}
                >
                  {children}
                </blockquote>
              )
            },
          }}
        >
          {markdownContent}
        </ReactMarkdown>
      </Box>
      {updateState && (
        <LinearProgress
          variant={total > 0 ? 'determinate' : 'indeterminate'}
          value={progress}
          className="mt-4"
        />
      )}
    </BaseDialog>
  )
}
