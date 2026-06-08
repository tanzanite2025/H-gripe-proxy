import type {
  ColumnSizingState,
  Updater,
  VisibilityState,
} from '@tanstack/react-table'
import type { ReactNode } from 'react'

import type {
  ConnectionTableField,
  ConnectionViewMode,
} from '../connection-page-model'

export interface ConnectionTableProps {
  connections: IConnectionsItem[]
  viewMode: ConnectionViewMode
  onShowDetail: (data: IConnectionsItem) => void
  columnManagerOpen: boolean
  onCloseColumnManager: () => void
}

export type ColumnField = ConnectionTableField | 'actions'

export interface BaseConnectionColumn {
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

export interface ConnectionColumnMeta {
  align?: 'left' | 'right'
  field: ColumnField
  label: string
}

export interface ConnectionColumnPreferences {
  columnWidths: ColumnSizingState
  columnVisibilityModel: VisibilityState
  columnOrder: string[]
  handleColumnSizingChange: (updater: Updater<ColumnSizingState>) => void
  handleColumnVisibilityChange: (updater: Updater<VisibilityState>) => void
  handleColumnOrderChange: (updater: Updater<string[]>) => void
}
