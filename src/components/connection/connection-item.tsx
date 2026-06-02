import { useLockFn } from 'ahooks'
import dayjs from 'dayjs'
import { X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { closeConnection } from 'tauri-plugin-mihomo-api'

import {
  getConnectionViewSpec,
  type ConnectionViewMode,
} from '@/components/connection/connection-page-model'
import { buildConnectionViewModel } from '@/components/connection/connection-view-model'
import { IconButton } from '@/components/tailwind/IconButton'
import { ListItem, ListItemText } from '@/components/tailwind/List'
import parseTraffic from '@/utils/format'

interface Props {
  value: IConnectionsItem
  viewMode: ConnectionViewMode
  onShowDetail?: () => void
}

export const ConnectionItem = (props: Props) => {
  const { value, viewMode, onShowDetail } = props

  const { id, start, curUpload, curDownload } = value
  const { t } = useTranslation()
  const viewModel = buildConnectionViewModel(value)
  const closed = viewMode === 'closed'
  const { listMetaFields } = getConnectionViewSpec(viewMode)

  const onDelete = useLockFn(async () => closeConnection(id))
  const showTraffic = (curUpload ?? 0) >= 100 || (curDownload ?? 0) >= 100

  const hasField = (field: (typeof listMetaFields)[number]) =>
    listMetaFields.includes(field)

  return (
    <ListItem
      className="border-b border-divider"
      secondaryAction={
        !closed && (
          <IconButton
            color="inherit"
            onClick={onDelete}
            title={t('connections.components.actions.closeConnection')}
            aria-label={t('connections.components.actions.closeConnection')}
          >
            <X className="h-4 w-4" />
          </IconButton>
        )
      }
    >
      <ListItemText
        className="select-text cursor-pointer"
        primary={viewModel.title}
        onClick={onShowDetail}
        secondary={
          <div className="flex flex-wrap">
            {hasField('network') && (
              <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1 uppercase text-success">
                {viewModel.network}
              </span>
            )}

            {hasField('type') && (
              <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1">
                {viewModel.typeLabel}
              </span>
            )}

            {hasField('process') && !!viewModel.processLabel && (
              <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1">
                {viewModel.processLabel}
              </span>
            )}

            {hasField('rule') && (
              <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1">
                {viewModel.ruleLabel}
              </span>
            )}

            {hasField('chains') && viewModel.hasChains && (
              <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1">
                {viewModel.chains}
              </span>
            )}

            {hasField('time') && (
              <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1">
                {dayjs(start).fromNow()}
              </span>
            )}

            {hasField('speed') && showTraffic && (
              <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1">
                {parseTraffic(curUpload!)} / {parseTraffic(curDownload!)}
              </span>
            )}

            {hasField('download') && (
              <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1">
                DL {parseTraffic(value.download).join(' ')}
              </span>
            )}

            {hasField('upload') && (
              <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1">
                UL {parseTraffic(value.upload).join(' ')}
              </span>
            )}
          </div>
        }
      />
    </ListItem>
  )
}
