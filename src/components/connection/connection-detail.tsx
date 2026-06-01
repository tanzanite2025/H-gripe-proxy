import { useLockFn } from 'ahooks'
import dayjs from 'dayjs'
import { useImperativeHandle, useState, type Ref } from 'react'
import { useTranslation } from 'react-i18next'
import { closeConnection } from 'tauri-plugin-mihomo-api'

import {
  getConnectionViewSpec,
  type ConnectionDetailField,
  type ConnectionViewMode,
} from '@/components/connection/connection-page-model'
import { Button } from '@/components/tailwind/Button'
import { Snackbar } from '@/components/tailwind/Snackbar'
import parseTraffic from '@/utils/format'

export interface ConnectionDetailRef {
  open: (detail: IConnectionsItem, viewMode: ConnectionViewMode) => void
}

export function ConnectionDetail({ ref }: { ref?: Ref<ConnectionDetailRef> }) {
  const [open, setOpen] = useState(false)
  const [detail, setDetail] = useState<IConnectionsItem>(null!)
  const [viewMode, setViewMode] = useState<ConnectionViewMode>('active')

  useImperativeHandle(ref, () => ({
    open: (detail: IConnectionsItem, nextViewMode: ConnectionViewMode) => {
      if (open) return
      setOpen(true)
      setDetail(detail)
      setViewMode(nextViewMode)
    },
  }))

  const onClose = () => setOpen(false)

  return (
    <Snackbar
      anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
      open={open}
      onClose={onClose}
      className="max-w-[520px] max-h-[480px] overflow-y-auto bg-paper text-text-primary"
      message={
        detail ? (
          <InnerConnectionDetail
            data={detail}
            viewMode={viewMode}
            onClose={onClose}
          />
        ) : null
      }
    />
  )
}

interface InnerProps {
  data: IConnectionsItem
  viewMode: ConnectionViewMode
  onClose?: () => void
}

const InnerConnectionDetail = ({ data, viewMode, onClose }: InnerProps) => {
  const { t } = useTranslation()
  const { metadata, rulePayload } = data
  const chains = [...data.chains].reverse().join(' / ')
  const rule = rulePayload ? `${data.rule}(${rulePayload})` : data.rule
  const host = metadata.host
    ? `${metadata.host}:${metadata.destinationPort}`
    : `${metadata.remoteDestination}:${metadata.destinationPort}`
  const Destination = metadata.destinationIP
    ? metadata.destinationIP
    : metadata.remoteDestination
  const closed = viewMode === 'closed'
  const { detailFields } = getConnectionViewSpec(viewMode)

  const detailFieldMap: Record<
    ConnectionDetailField,
    { label: string; value: string }
  > = {
    host: { label: t('connections.components.fields.host'), value: host },
    download: {
      label: t('shared.labels.downloaded'),
      value: parseTraffic(data.download).join(' '),
    },
    upload: {
      label: t('shared.labels.uploaded'),
      value: parseTraffic(data.upload).join(' '),
    },
    dlSpeed: {
      label: t('connections.components.fields.dlSpeed'),
      value: `${parseTraffic(data.curDownload ?? -1).join(' ')}/s`,
    },
    ulSpeed: {
      label: t('connections.components.fields.ulSpeed'),
      value: `${parseTraffic(data.curUpload ?? -1).join(' ')}/s`,
    },
    chains: {
      label: t('connections.components.fields.chains'),
      value: chains,
    },
    rule: { label: t('connections.components.fields.rule'), value: rule },
    process: {
      label: t('connections.components.fields.process'),
      value: `${metadata.process ?? ''}${metadata.processPath ? `(${metadata.processPath})` : ''}`,
    },
    time: {
      label: t('connections.components.fields.time'),
      value: dayjs(data.start).fromNow(),
    },
    source: {
      label: t('connections.components.fields.source'),
      value: `${metadata.sourceIP}:${metadata.sourcePort}`,
    },
    destination: {
      label: t('connections.components.fields.destination'),
      value: Destination ?? '',
    },
    destinationPort: {
      label: t('connections.components.fields.destinationPort'),
      value: `${metadata.destinationPort}`,
    },
    type: {
      label: t('connections.components.fields.type'),
      value: `${metadata.type}(${metadata.network})`,
    },
  }

  const information = detailFields.map((field) => detailFieldMap[field])

  const onDelete = useLockFn(async () => closeConnection(data.id))

  return (
    <div className="select-text text-text-secondary">
      {information.map((each) => (
        <div key={each.label}>
          <b>{each.label}</b>
          <span className="break-all text-text-primary">
            : {each.value}
          </span>
        </div>
      ))}

      {!closed && (
        <div className="text-right">
          <Button
            variant="contained"
            title={t('connections.components.actions.closeConnection')}
            onClick={() => {
              onDelete()
              onClose?.()
            }}
          >
            {t('connections.components.actions.closeConnection')}
          </Button>
        </div>
      )}
    </div>
  )
}
