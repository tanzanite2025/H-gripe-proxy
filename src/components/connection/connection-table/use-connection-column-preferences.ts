import type {
  ColumnOrderState,
  ColumnSizingState,
  Updater,
  VisibilityState,
} from '@tanstack/react-table'
import { useLocalStorage } from 'foxact/use-local-storage'
import { useCallback, useEffect } from 'react'

import { reconcileColumnOrder } from './helpers'
import type {
  BaseConnectionColumn,
  ConnectionColumnPreferences,
} from './types'

const objectStorageOptions = {
  serializer: JSON.stringify,
  deserializer: (value: string) => {
    try {
      const parsed = JSON.parse(value)
      if (parsed && typeof parsed === 'object') {
        return parsed
      }
    } catch (error) {
      console.warn('Failed to parse stored connection-table object', error)
    }

    return {}
  },
}

const arrayStorageOptions = {
  serializer: JSON.stringify,
  deserializer: (value: string) => {
    try {
      const parsed = JSON.parse(value)
      if (Array.isArray(parsed)) {
        return parsed
      }
    } catch (error) {
      console.warn('Failed to parse stored connection-table array', error)
    }

    return []
  },
}

export function useConnectionColumnPreferences(
  baseColumns: BaseConnectionColumn[],
): ConnectionColumnPreferences {
  const [columnWidths, setColumnWidths] = useLocalStorage<ColumnSizingState>(
    'connection-table-widths',
    {},
  )
  const [columnVisibilityModel, setColumnVisibilityModel] =
    useLocalStorage<VisibilityState>(
      'connection-table-visibility',
      {},
      objectStorageOptions,
    )
  const [columnOrder, setColumnOrder] = useLocalStorage<string[]>(
    'connection-table-order',
    [],
    arrayStorageOptions,
  )

  useEffect(() => {
    setColumnOrder((previousValue) => {
      const baseFields = baseColumns.map((column) => column.field)
      const previous = Array.isArray(previousValue) ? previousValue : []
      const reconciled = reconcileColumnOrder(previous, baseFields)

      if (
        reconciled.length === previous.length &&
        reconciled.every((field, index) => field === previous[index])
      ) {
        return previousValue
      }

      return reconciled
    })
  }, [baseColumns, setColumnOrder])

  const handleColumnSizingChange = useCallback(
    (updater: Updater<ColumnSizingState>) => {
      setColumnWidths((previousValue) => {
        const previousState = previousValue ?? {}
        const nextState =
          typeof updater === 'function' ? updater(previousState) : updater
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

  const handleColumnVisibilityChange = useCallback(
    (updater: Updater<VisibilityState>) => {
      setColumnVisibilityModel((previousValue) => {
        const current = previousValue ?? {}
        const nextState =
          typeof updater === 'function' ? updater(current) : updater

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
    (updater: Updater<ColumnOrderState>) => {
      setColumnOrder((previousValue) => {
        const current = Array.isArray(previousValue) ? previousValue : []
        const nextState =
          typeof updater === 'function' ? updater(current) : updater
        const baseFields = baseColumns.map((column) => column.field)
        return reconcileColumnOrder(nextState, baseFields)
      })
    },
    [baseColumns, setColumnOrder],
  )

  return {
    columnWidths,
    columnVisibilityModel: columnVisibilityModel ?? {},
    columnOrder,
    handleColumnSizingChange,
    handleColumnVisibilityChange,
    handleColumnOrderChange,
  }
}
