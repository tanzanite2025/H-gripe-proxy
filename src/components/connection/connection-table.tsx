import {
  ColumnDef,
  ColumnOrderState,
  ColumnSizingState,
  getCoreRowModel,
  getSortedRowModel,
  SortingState,
  Updater,
  useReactTable,
  VisibilityState,
} from '@tanstack/react-table'
import dayjs from 'dayjs'
import { useLocalStorage } from 'foxact/use-local-storage'
import { X } from 'lucide-react'
import {
  memo,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
  type ReactNode,
} from 'react'
import { useTranslation } from 'react-i18next'
import { closeConnection } from 'tauri-plugin-mihomo-api'

import { IconButton } from '@/components/tailwind/IconButton'
import parseTraffic from '@/utils/format'
import { truncateStr } from '@/utils/format'

import {
  getConnectionViewSpec,
  type ConnectionTableField,
  type ConnectionViewMode,
} from './connection-page-model'
import { createCloseConnectionAction } from './connection-actions'
import { ConnectionColumnManager } from './connection-column-manager'
import { ConnectionTableUI } from './connection-table-ui'

type TickListener = () => void
let _tickNow = Date.now()
const _tickListeners = new Set<TickListener>()
let _tickTimer: ReturnType<typeof setInterval> | null = null

const _startTick = () => {
  if (_tickTimer !== null) return
  _tickTimer = setInterval(() => {
    _tickNow = Date.now()
    _tickListeners.forEach((fn) => fn())
  }, 5000)
}

const _stopTick = () => {
  if (_tickListeners.size === 0 && _tickTimer !== null) {
    clearInterval(_tickTimer)
    _tickTimer = null
  }
}

const tickStore = {
  subscribe: (listener: TickListener) => {
    _tickListeners.add(listener)
    _startTick()
    return () => {
      _tickListeners.delete(listener)
      _stopTick()
    }
  },
  getSnapshot: () => _tickNow,
}

interface RelativeTimeCellProps {
  start: string
}

const RelativeTimeCell = memo(function RelativeTimeCell({
  start,
}: RelativeTimeCellProps) {
  const now = useSyncExternalStore(tickStore.subscribe, tickStore.getSnapshot)
  return <>{dayjs(start).from(now)}</>
})

const reconcileColumnOrder = (
  storedOrder: string[],
  baseFields: string[],
): string[] => {
  const filtered = storedOrder.filter((field) => baseFields.includes(field))
  const missing = baseFields.filter((field) => !filtered.includes(field))
  return [...filtered, ...missing]
}

type ColumnField = ConnectionTableField | 'actions'

const getConnectionCellValue = (field: ColumnField, each: IConnectionsItem) => {
  const { metadata, rulePayload } = each

  switch (field) {
    case 'host':
      return metadata.host
        ? `${metadata.host}:${metadata.destinationPort}`
        : `${metadata.remoteDestination}:${metadata.destinationPort}`
    case 'download':
      return each.download
    case 'upload':
      return each.upload
    case 'dlSpeed':
      return each.curDownload
    case 'ulSpeed':
      return each.curUpload
    case 'chains':
      return [...each.chains].reverse().join(' / ')
    case 'rule':
      return rulePayload ? `${each.rule}(${rulePayload})` : each.rule
    case 'process':
      return truncateStr(metadata.process || metadata.processPath)
    case 'time':
      return each.start
    case 'source':
      return `${metadata.sourceIP}:${metadata.sourcePort}`
    case 'remoteDestination':
      return metadata.destinationIP
        ? `${metadata.destinationIP}:${metadata.destinationPort}`
        : `${metadata.remoteDestination}:${metadata.destinationPort}`
    case 'type':
      return `${metadata.type}(${metadata.network})`
    case 'actions':
      return ''
    default:
      return ''
  }
}

interface Props {
  connections: IConnectionsItem[]
  viewMode: ConnectionViewMode
  onShowDetail: (data: IConnectionsItem) => void
  columnManagerOpen: boolean
  onCloseColumnManager: () => void
}

export const ConnectionTable = (props: Props) => {
  const {
    connections,
    viewMode,
    onShowDetail: rawOnShowDetail,
    columnManagerOpen,
    onCloseColumnManager,
  } = props
  const onShowDetailRef = useRef(rawOnShowDetail)
  onShowDetailRef.current = rawOnShowDetail
  const onShowDetail = useCallback(
    (data: IConnectionsItem) => onShowDetailRef.current(data),
    [],
  )
  const { t } = useTranslation()
  const closed = viewMode === 'closed'
  const { tableFields } = getConnectionViewSpec(viewMode)
  const [columnWidths, setColumnWidths] = useLocalStorage<ColumnSizingState>(
    'connection-table-widths',
    {},
  )

  const [columnVisibilityModel, setColumnVisibilityModel] =
    useLocalStorage<VisibilityState>(
      'connection-table-visibility',
      {},
      {
        serializer: JSON.stringify,
        deserializer: (value) => {
          try {
            const parsed = JSON.parse(value)
            if (parsed && typeof parsed === 'object') return parsed
          } catch (err) {
            console.warn('Failed to parse connection-table-visibility', err)
          }
          return {}
        },
      },
    )

  const [columnOrder, setColumnOrder] = useLocalStorage<string[]>(
    'connection-table-order',
    [],
    {
      serializer: JSON.stringify,
      deserializer: (value) => {
        try {
          const parsed = JSON.parse(value)
          if (Array.isArray(parsed)) return parsed
        } catch (err) {
          console.warn('Failed to parse connection-table-order', err)
        }
        return []
      },
    },
  )

  interface BaseColumn {
    field: ColumnField
    headerName: string
    width?: number
    minWidth?: number
    align?: 'left' | 'right'
    cell?: (row: IConnectionsItem) => ReactNode
    enableSorting?: boolean
    enableResizing?: boolean
    canHide?: boolean
  }

  const baseColumns = useMemo<BaseColumn[]>(() => {
    const columnRegistry: Record<ConnectionTableField, BaseColumn> = {
      host: {
        field: 'host',
        headerName: t('connections.components.fields.host'),
        width: 180,
        minWidth: 140,
      },
      download: {
        field: 'download',
        headerName: t('shared.labels.downloaded'),
        width: 76,
        minWidth: 60,
        align: 'right',
        cell: (row) => parseTraffic(row.download).join(' '),
      },
      upload: {
        field: 'upload',
        headerName: t('shared.labels.uploaded'),
        width: 76,
        minWidth: 60,
        align: 'right',
        cell: (row) => parseTraffic(row.upload).join(' '),
      },
      dlSpeed: {
        field: 'dlSpeed',
        headerName: t('connections.components.fields.dlSpeed'),
        width: 76,
        minWidth: 60,
        align: 'right',
        cell: (row) => `${parseTraffic(row.curDownload).join(' ')}/s`,
      },
      ulSpeed: {
        field: 'ulSpeed',
        headerName: t('connections.components.fields.ulSpeed'),
        width: 76,
        minWidth: 60,
        align: 'right',
        cell: (row) => `${parseTraffic(row.curUpload).join(' ')}/s`,
      },
      chains: {
        field: 'chains',
        headerName: t('connections.components.fields.chains'),
        width: 280,
        minWidth: 160,
      },
      rule: {
        field: 'rule',
        headerName: t('connections.components.fields.rule'),
        width: 220,
        minWidth: 160,
      },
      process: {
        field: 'process',
        headerName: t('connections.components.fields.process'),
        width: 180,
        minWidth: 140,
      },
      time: {
        field: 'time',
        headerName: t('connections.components.fields.time'),
        width: 100,
        minWidth: 80,
        align: 'right',
      },
      source: {
        field: 'source',
        headerName: t('connections.components.fields.source'),
        width: 160,
        minWidth: 120,
      },
      remoteDestination: {
        field: 'remoteDestination',
        headerName: t('connections.components.fields.destination'),
        width: 160,
        minWidth: 120,
      },
      type: {
        field: 'type',
        headerName: t('connections.components.fields.type'),
        width: 120,
        minWidth: 80,
      },
    }

    const columns = tableFields.map((field) => columnRegistry[field])

    if (!closed) {
      columns.push({
        field: 'actions',
        headerName: t('connections.components.actions.closeConnection'),
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
            onCloseConnection: closeConnection,
          })

          if (!action) return null

          return (
            <IconButton
              size="small"
              title={t('connections.components.actions.closeConnection')}
              aria-label={t('connections.components.actions.closeConnection')}
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
  }, [closed, t, tableFields])

  useEffect(() => {
    setColumnOrder((prevValue) => {
      const baseFields = baseColumns.map((col) => col.field)
      const prev = Array.isArray(prevValue) ? prevValue : []
      const reconciled = reconcileColumnOrder(prev, baseFields)
      if (
        reconciled.length === prev.length &&
        reconciled.every((field, i) => field === prev[i])
      ) {
        return prevValue
      }
      return reconciled
    })
  }, [baseColumns, setColumnOrder])

  const handleColumnVisibilityChange = useCallback(
    (update: Updater<VisibilityState>) => {
      setColumnVisibilityModel((prev) => {
        const current = prev ?? {}
        const nextState =
          typeof update === 'function' ? update(current) : update

        const visibleCount = baseColumns.reduce((count, column) => {
          const isVisible = (nextState[column.field] ?? true) !== false
          return count + (isVisible ? 1 : 0)
        }, 0)

        if (visibleCount === 0) {
          return current
        }

        const sanitized: VisibilityState = {}
        baseColumns.forEach((column) => {
          if (nextState[column.field] === false) {
            sanitized[column.field] = false
          }
        })
        return sanitized
      })
    },
    [baseColumns, setColumnVisibilityModel],
  )

  const handleColumnOrderChange = useCallback(
    (update: Updater<ColumnOrderState>) => {
      setColumnOrder((prev) => {
        const current = Array.isArray(prev) ? prev : []
        const nextState =
          typeof update === 'function' ? update(current) : update
        const baseFields = baseColumns.map((col) => col.field)
        return reconcileColumnOrder(nextState, baseFields)
      })
    },
    [baseColumns, setColumnOrder],
  )

  const [sorting, setSorting] = useState<SortingState>([])

  const columnDefs = useMemo<ColumnDef<IConnectionsItem>[]>(() => {
    return baseColumns.map((column) => {
      let cell: ColumnDef<IConnectionsItem>['cell']
      if (column.field === 'time') {
        cell = (ctx) => <RelativeTimeCell start={ctx.row.original.start} />
      } else if (column.cell) {
        const renderCell = column.cell
        cell = (ctx) => renderCell(ctx.row.original)
      } else {
        cell = (ctx) =>
          ctx.row.original
            ? (getConnectionCellValue(
                column.field,
                ctx.row.original,
              ) as ReactNode)
            : null
      }

      return {
        id: column.field,
        accessorFn: (row) => getConnectionCellValue(column.field, row),
        header: column.headerName,
        size: column.width,
        minSize: column.minWidth,
        meta: {
          align: column.align ?? 'left',
          field: column.field,
          label: column.headerName,
        },
        cell,
        enableSorting: column.enableSorting ?? true,
        enableHiding: column.canHide ?? true,
        enableResizing: column.enableResizing ?? true,
      } satisfies ColumnDef<IConnectionsItem>
    })
  }, [baseColumns])

  const handleColumnSizingChange = useCallback(
    (updater: Updater<ColumnSizingState>) => {
      setColumnWidths((prev) => {
        const prevState = prev ?? {}
        const nextState =
          typeof updater === 'function' ? updater(prevState) : updater
        const sanitized: ColumnSizingState = {}
        Object.entries(nextState).forEach(([key, size]) => {
          if (typeof size === 'number' && Number.isFinite(size)) {
            sanitized[key] = size
          }
        })
        return sanitized
      })
    },
    [setColumnWidths],
  )

  const table = useReactTable({
    data: connections,
    state: {
      columnVisibility: columnVisibilityModel ?? {},
      columnSizing: columnWidths,
      columnOrder,
      sorting,
    },
    initialState: {
      columnOrder: baseColumns.map((col) => col.field),
    },
    defaultColumn: {
      minSize: 80,
      enableResizing: true,
    },
    columnResizeMode: 'onChange',
    enableSortingRemoval: true,
    getRowId: (row) => row.id,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: sorting.length ? getSortedRowModel() : undefined,
    onSortingChange: setSorting,
    onColumnSizingChange: handleColumnSizingChange,
    onColumnVisibilityChange: handleColumnVisibilityChange,
    onColumnOrderChange: handleColumnOrderChange,
    columns: columnDefs,
  })

  const handleManagerOrderChange = useCallback(
    (order: string[]) => {
      const baseFields = baseColumns.map((col) => col.field)
      table.setColumnOrder(reconcileColumnOrder(order, baseFields))
    },
    [baseColumns, table],
  )

  const handleResetColumns = useCallback(() => {
    table.resetColumnVisibility()
    table.resetColumnOrder()
  }, [table])

  const managerColumns = table.getAllLeafColumns()

  return (
    <>
      <ConnectionTableUI table={table} onShowDetail={onShowDetail} />
      <ConnectionColumnManager
        open={columnManagerOpen}
        columns={managerColumns}
        onClose={onCloseColumnManager}
        onOrderChange={handleManagerOrderChange}
        onReset={handleResetColumns}
      />
    </>
  )
}
