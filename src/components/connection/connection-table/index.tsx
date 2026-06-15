import type { ColumnDef, SortingState } from '@tanstack/react-table'
import {
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from '@tanstack/react-table'
import { useCallback, useMemo, useRef, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'

import { ConnectionColumnManager } from '../connection-column-manager'
import { getConnectionViewSpec } from '../connection-page-model'
import { ConnectionTableUI } from '../connection-table-ui'

import { buildConnectionBaseColumns } from './columns'
import { getConnectionCellValue, reconcileColumnOrder } from './helpers'
import { RelativeTimeCell } from './relative-time-cell'
import type {
  ConnectionColumnMeta,
  ConnectionTableProps,
} from './types'
import { useConnectionColumnPreferences } from './use-connection-column-preferences'

export const ConnectionTable = ({
  connections,
  viewMode,
  onShowDetail: rawOnShowDetail,
  columnManagerOpen,
  onCloseColumnManager,
}: ConnectionTableProps) => {
  const onShowDetailRef = useRef(rawOnShowDetail)
  onShowDetailRef.current = rawOnShowDetail

  const onShowDetail = useCallback(
    (connection: IConnectionsItem) => onShowDetailRef.current(connection),
    [],
  )

  const { t } = useTranslation()
  const closed = viewMode === 'closed'
  const { tableFields } = getConnectionViewSpec(viewMode)
  const [sorting, setSorting] = useState<SortingState>([])

  const labels = useMemo(
    () => ({
      closeConnection: t('connections.components.actions.closeConnection'),
      downloaded: t('shared.labels.downloaded'),
      uploaded: t('shared.labels.uploaded'),
      host: t('connections.components.fields.host'),
      dlSpeed: t('connections.components.fields.dlSpeed'),
      ulSpeed: t('connections.components.fields.ulSpeed'),
      chains: t('connections.components.fields.chains'),
      rule: t('connections.components.fields.rule'),
      process: t('connections.components.fields.process'),
      time: t('connections.components.fields.time'),
      source: t('connections.components.fields.source'),
      destination: t('connections.components.fields.destination'),
      type: t('connections.components.fields.type'),
    }),
    [t],
  )

  const baseColumns = useMemo(
    () =>
      buildConnectionBaseColumns({
        closed,
        labels,
        tableFields,
      }),
    [closed, labels, tableFields],
  )
  const baseFields = useMemo(
    () => baseColumns.map((column) => column.field),
    [baseColumns],
  )

  const {
    columnWidths,
    columnVisibilityModel,
    columnOrder,
    handleColumnSizingChange,
    handleColumnVisibilityChange,
    handleColumnOrderChange,
  } = useConnectionColumnPreferences(baseColumns)

  const columnDefs = useMemo<ColumnDef<IConnectionsItem>[]>(() => {
    return baseColumns.map((column) => {
      let cell: ColumnDef<IConnectionsItem>['cell']

      if (column.field === 'time') {
        cell = (context) => (
          <RelativeTimeCell start={context.row.original.start} />
        )
      } else if (column.cell) {
        const renderCell = column.cell
        cell = (context) => renderCell(context.row.original)
      } else {
        cell = (context) =>
          context.row.original
            ? (getConnectionCellValue(
                column.field,
                context.row.original,
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
        } satisfies ConnectionColumnMeta,
        cell,
        enableSorting: column.enableSorting ?? true,
        enableHiding: column.canHide ?? true,
        enableResizing: column.enableResizing ?? true,
      } satisfies ColumnDef<IConnectionsItem>
    })
  }, [baseColumns])

  const table = useReactTable({
    data: connections,
    state: {
      columnVisibility: columnVisibilityModel,
      columnSizing: columnWidths,
      columnOrder,
      sorting,
    },
    initialState: {
      columnOrder: baseFields,
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
      table.setColumnOrder(reconcileColumnOrder(order, baseFields))
    },
    [baseFields, table],
  )

  const handleResetColumns = useCallback(() => {
    table.resetColumnVisibility()
    table.resetColumnOrder()
  }, [table])

  return (
    <>
      <ConnectionTableUI table={table} onShowDetail={onShowDetail} />
      <ConnectionColumnManager
        open={columnManagerOpen}
        columns={table.getAllLeafColumns()}
        onClose={onCloseColumnManager}
        onOrderChange={handleManagerOrderChange}
        onReset={handleResetColumns}
      />
    </>
  )
}
