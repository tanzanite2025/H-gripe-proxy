import { useLockFn } from 'ahooks'
import { Copy, Download, Link2, Share2 } from 'lucide-react'
import { QRCodeSVG } from 'qrcode.react'
import { useMemo, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/tailwind/Button'
import {
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
} from '@/components/tailwind/Dialog'
import { showNotice } from '@/services/notice-service'

interface Props {
  open: boolean
  value: string
  title?: string
  subject?: string
  fileName?: string
  onClose: () => void
}

const DEFAULT_DOWNLOAD_NAME = 'profile-share-qr'
const INVALID_FILE_NAME_CHARS = '<>:"/\\|?*'

const sanitizeFileName = (value?: string) => {
  const normalized = (value ?? '')
    .trim()
    .split('')
    .map((char) => {
      const code = char.charCodeAt(0)
      return code <= 31 || INVALID_FILE_NAME_CHARS.includes(char) ? '-' : char
    })
    .join('')
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '')

  return normalized || DEFAULT_DOWNLOAD_NAME
}

const fallbackCopyText = (value: string) => {
  const textArea = document.createElement('textarea')
  textArea.value = value
  textArea.setAttribute('readonly', 'true')
  textArea.style.position = 'fixed'
  textArea.style.opacity = '0'
  textArea.style.pointerEvents = 'none'

  document.body.appendChild(textArea)
  textArea.select()

  const copied = document.execCommand('copy')
  document.body.removeChild(textArea)

  if (!copied) {
    throw new Error('Failed to copy share link')
  }
}

export const QrViewer = ({
  open,
  value,
  title,
  subject,
  fileName,
  onClose,
}: Props) => {
  const { t } = useTranslation()
  const qrCodeContainerRef = useRef<HTMLDivElement | null>(null)

  const resolvedTitle = title ?? t('profiles.modals.qrViewer.title')
  const copyButtonLabel = t('settings.sections.externalController.tooltips.copy')
  const downloadButtonLabel = `${t('home.components.traffic.legends.download')} SVG`
  const resolvedFileName = useMemo(
    () => `${sanitizeFileName(fileName ?? subject ?? title)}.svg`,
    [fileName, subject, title],
  )

  const handleCopyLink = useLockFn(async () => {
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(value)
      } else {
        fallbackCopyText(value)
      }

      showNotice.success('shared.feedback.notifications.common.copySuccess')
    } catch (error) {
      showNotice.error(error)
    }
  })

  const handleDownloadQr = useLockFn(async () => {
    try {
      const svgElement = qrCodeContainerRef.current?.querySelector('svg')
      if (!(svgElement instanceof SVGSVGElement)) {
        throw new Error('QR code is not ready yet')
      }

      const serializedSvg = new XMLSerializer().serializeToString(svgElement)
      const svgBlob = new Blob([serializedSvg], {
        type: 'image/svg+xml;charset=utf-8',
      })
      const objectUrl = URL.createObjectURL(svgBlob)
      const anchor = document.createElement('a')

      anchor.href = objectUrl
      anchor.download = resolvedFileName
      document.body.appendChild(anchor)
      anchor.click()
      document.body.removeChild(anchor)
      URL.revokeObjectURL(objectUrl)

      showNotice.success('shared.feedback.notifications.saved')
    } catch (error) {
      showNotice.error(error)
    }
  })

  return (
    <Dialog open={open} onClose={onClose} maxWidth="sm" fullWidth>
      <DialogTitle className="pb-2">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.14em] text-primary">
              <Share2 className="h-3.5 w-3.5" />
              <span>{resolvedTitle}</span>
            </div>
            <div className="mt-1 text-xs leading-5 text-text-secondary">
              {resolvedFileName}
            </div>
            {subject && (
              <div
                className="mt-2 truncate text-base font-semibold text-text-primary"
                title={subject}
              >
                {subject}
              </div>
            )}
          </div>
        </div>
      </DialogTitle>

      <DialogContent className="overflow-y-visible pb-1 pt-1">
        <div className="overflow-hidden rounded-[28px] border border-white/10 bg-[radial-gradient(circle_at_top,_rgba(20,184,166,0.18),_transparent_52%),linear-gradient(180deg,rgba(255,255,255,0.05),rgba(255,255,255,0.02))] shadow-[0_20px_50px_rgba(15,23,42,0.26)]">
          <div className="grid gap-4 p-4 sm:grid-cols-[268px_minmax(0,1fr)]">
            <div
              ref={qrCodeContainerRef}
              className="flex items-center justify-center rounded-[24px] bg-white p-4 shadow-[inset_0_1px_0_rgba(255,255,255,0.65),0_18px_40px_rgba(15,23,42,0.18)]"
            >
              <QRCodeSVG
                value={value}
                size={256}
                level="M"
                marginSize={2}
                bgColor="#FFFFFF"
                fgColor="#111827"
              />
            </div>

            <div className="flex min-w-0 flex-col gap-3">
              <div className="rounded-2xl border border-white/10 bg-black/15 p-3">
                <div className="mb-2 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.12em] text-text-secondary">
                  <Link2 className="h-3.5 w-3.5" />
                  <span>URL</span>
                </div>
                <div
                  className="max-h-32 overflow-y-auto break-all rounded-xl bg-black/20 px-3 py-2 font-mono text-[11px] leading-5 text-text-primary"
                  title={value}
                >
                  {value}
                </div>
              </div>

              <div className="grid gap-2 sm:grid-cols-2">
                <Button
                  onClick={() => {
                    void handleCopyLink()
                  }}
                  variant="contained"
                  color="primary"
                  startIcon={<Copy className="h-4 w-4" />}
                  fullWidth
                >
                  {copyButtonLabel}
                </Button>
                <Button
                  onClick={() => {
                    void handleDownloadQr()
                  }}
                  variant="outlined"
                  color="inherit"
                  startIcon={<Download className="h-4 w-4" />}
                  fullWidth
                >
                  {downloadButtonLabel}
                </Button>
              </div>
            </div>
          </div>
        </div>
      </DialogContent>

      <DialogActions className="justify-end pt-5">
        <Button onClick={onClose} variant="outlined">
          {t('shared.actions.close')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
