import { X } from 'lucide-react'

import { IconButton } from '@/components/tailwind/IconButton'
import { closeRuntimeConnection } from '@/services/connection-runtime'
import parseTraffic from '@/utils/format'

import { createCloseConnectionAction } from '../connection-actions'
import type { ConnectionTableField } from '../connection-page-model'

import type { BaseConnectionColumn } from './types'

interface ConnectionColumnLabels {
  closeConnection: string
  downloaded: string
  uploaded: string
  host: string
  dlSpeed: string
  ulSpeed: string
  chains: string
  rule: string
  process: string
  time: string
  source: string
  destination: string
  type: string
}

interface BuildConnectionBaseColumnsOptions {
  closed: boolean
  labels: ConnectionColumnLabels
  tableFields: ConnectionTableField[]
}

export const buildConnectionBaseColumns = ({
  closed,
  labels,
  tableFields,
}: BuildConnectionBaseColumnsOptions): BaseConnectionColumn[] => {
  const columnRegistry: Record<ConnectionTableField, BaseConnectionColumn> = {
    host: {
      field: 'host',
      headerName: labels.host,
      width: 180,
      minWidth: 140,
    },
    download: {
      field: 'download',
      headerName: labels.downloaded,
      width: 76,
      minWidth: 60,
      align: 'right',
      cell: (row) => parseTraffic(row.download).join(' '),
    },
    upload: {
      field: 'upload',
      headerName: labels.uploaded,
      width: 76,
      minWidth: 60,
      align: 'right',
      cell: (row) => parseTraffic(row.upload).join(' '),
    },
    dlSpeed: {
      field: 'dlSpeed',
      headerName: labels.dlSpeed,
      width: 76,
      minWidth: 60,
      align: 'right',
      cell: (row) => `${parseTraffic(row.curDownload).join(' ')}/s`,
    },
    ulSpeed: {
      field: 'ulSpeed',
      headerName: labels.ulSpeed,
      width: 76,
      minWidth: 60,
      align: 'right',
      cell: (row) => `${parseTraffic(row.curUpload).join(' ')}/s`,
    },
    chains: {
      field: 'chains',
      headerName: labels.chains,
      width: 280,
      minWidth: 160,
    },
    rule: {
      field: 'rule',
      headerName: labels.rule,
      width: 220,
      minWidth: 160,
    },
    process: {
      field: 'process',
      headerName: labels.process,
      width: 180,
      minWidth: 140,
    },
    time: {
      field: 'time',
      headerName: labels.time,
      width: 100,
      minWidth: 80,
      align: 'right',
    },
    source: {
      field: 'source',
      headerName: labels.source,
      width: 160,
      minWidth: 120,
    },
    remoteDestination: {
      field: 'remoteDestination',
      headerName: labels.destination,
      width: 160,
      minWidth: 120,
    },
    type: {
      field: 'type',
      headerName: labels.type,
      width: 120,
      minWidth: 80,
    },
  }

  const columns = tableFields.map((field) => columnRegistry[field])

  if (!closed) {
    columns.push({
      field: 'actions',
      headerName: labels.closeConnection,
      width: 72,
      minWidth: 72,
      align: 'right',
      enableSorting: false,
      enableResizing: false,
      canHide: false,
      cell: (row) => {
        const action = createCloseConnectionAction({
          closed,
          connectionId: row.id,
          onCloseConnection: closeRuntimeConnection,
        })

        if (!action) {
          return null
        }

        return (
          <IconButton
            size="small"
            title={labels.closeConnection}
            aria-label={labels.closeConnection}
            onClick={(event) => {
              event.stopPropagation()
              void action.onAction()
            }}
          >
            <X className="h-4 w-4" />
          </IconButton>
        )
      },
    })
  }

  return columns
}
