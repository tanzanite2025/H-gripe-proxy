import { X } from 'lucide-react'
import { useLockFn } from 'ahooks'
import dayjs from 'dayjs'
import { useTranslation } from 'react-i18next'
import { closeConnection } from 'tauri-plugin-mihomo-api'

import { IconButton } from '@/components/tailwind/IconButton'
import { ListItem, ListItemText } from '@/components/tailwind/List'
import parseTraffic from '@/utils/format'
import { cn } from '@/utils/cn'

interface Props {
  value: IConnectionsItem
  closed: boolean
  onShowDetail?: () => void
}

export const ConnectionItem = (props: Props) => {
  const { value, closed, onShowDetail } = props

  const { id, metadata, chains, start, curUpload, curDownload } = value
  const { t } = useTranslation()

  const onDelete = useLockFn(async () => closeConnection(id))
  const showTraffic = curUpload! >= 100 || curDownload! >= 100

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
        primary={metadata.host || metadata.destinationIP}
        onClick={onShowDetail}
        secondary={
          <div className="flex flex-wrap">
            <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1 uppercase text-success">
              {metadata.network}
            </span>

            <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1">
              {metadata.type}
            </span>

            {!!metadata.process && (
              <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1">
                {metadata.process}
              </span>
            )}

            {chains?.length > 0 && (
              <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1">
                {[...chains].reverse().join(' / ')}
              </span>
            )}

            <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1">
              {dayjs(start).fromNow()}
            </span>

            {showTraffic && (
              <span className="text-[10px] px-1 leading-tight border border-text-secondary/35 rounded mt-1 mr-1">
                {parseTraffic(curUpload!)} / {parseTraffic(curDownload!)}
              </span>
            )}
          </div>
        }
      />
    </ListItem>
  )
}
