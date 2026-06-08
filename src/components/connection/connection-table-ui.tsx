import { flexRender, Row, Table } from '@tanstack/react-table'
import { useVirtualizer } from '@tanstack/react-virtual'
import { memo, useCallback, useRef } from 'react'

import { cn } from '@/utils/cn'

const ROW_HEIGHT = 40
const SORT_INDICATORS = {
  asc: '^',
  desc: 'v',
} as const

interface RowComponentProps {
  row: Row<IConnectionsItem>
  virtualStart: number
  virtualSize: number
  onShowDetail: (data: IConnectionsItem) => void
}

const RowComponent = memo(
  function RowComponent({
    row,
    virtualStart,
    virtualSize,
    onShowDetail,
  }: RowComponentProps) {
    const handleClick = useCallback(
      () => onShowDetail(row.original),
      [onShowDetail, row.original],
    )

    return (
      <div
        className="flex absolute left-0 right-0 cursor-pointer border-b border-divider hover:bg-action-hover"
        style={{
          height: virtualSize,
          transform: `translateY(${virtualStart}px)`,
        }}
        onClick={handleClick}
      >
        {row.getVisibleCells().map((cell) => {
          const meta = cell.column.columnDef.meta as {
            align?: 'left' | 'right'
          }
          return (
            <div
              key={cell.id}
              className={cn(
                'box-border px-2 text-[13px] flex items-center whitespace-nowrap overflow-hidden text-ellipsis',
                meta?.align === 'right' ? 'justify-end' : 'justify-start',
              )}
              style={{
                flex: `0 0 ${cell.column.getSize()}px`,
                minWidth: cell.column.columnDef.minSize ?? 80,
                maxWidth: cell.column.columnDef.maxSize,
              }}
            >
              {flexRender(cell.column.columnDef.cell, cell.getContext())}
            </div>
          )
        })}
      </div>
    )
  },
  (prev, next) =>
    prev.row === next.row &&
    prev.virtualStart === next.virtualStart &&
    prev.virtualSize === next.virtualSize &&
    prev.onShowDetail === next.onShowDetail,
)

interface ConnectionTableUIProps {
  table: Table<IConnectionsItem>
  onShowDetail: (data: IConnectionsItem) => void
}

export const ConnectionTableUI = ({
  table,
  onShowDetail,
}: ConnectionTableUIProps) => {
  const rows = table.getRowModel().rows
  const tableContainerRef = useRef<HTMLDivElement | null>(null)
  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => tableContainerRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 4,
  })

  const virtualRows = rowVirtualizer.getVirtualItems()
  const totalSize = rowVirtualizer.getTotalSize()
  const tableWidth = table.getTotalSize()

  return (
    <div className="relative flex min-h-0 flex-1 flex-col font-sans">
      <div
        ref={tableContainerRef}
        className="scrollbar-thin scrollbar-h-2 flex-1 min-h-0 overflow-auto overscroll-contain rounded border-none"
      >
        <div className="min-w-full" style={{ width: tableWidth }}>
          <div className="sticky top-0 z-[2]">
            {table.getHeaderGroups().map((headerGroup) => (
              <div
                key={headerGroup.id}
                className="flex border-b border-divider bg-paper"
              >
                {headerGroup.headers.map((header) => {
                  if (header.isPlaceholder) {
                    return null
                  }

                  const meta = header.column.columnDef.meta as {
                    align?: 'left' | 'right'
                    field: string
                  }
                  const sortDirection = header.column.getIsSorted()

                  return (
                    <div
                      key={header.id}
                      className="relative box-border flex items-center text-[13px] font-semibold text-text-secondary select-none hover:bg-action-hover"
                      style={{
                        flex: `0 0 ${header.getSize()}px`,
                        minWidth: header.column.columnDef.minSize ?? 80,
                        maxWidth: header.column.columnDef.maxSize,
                      }}
                    >
                      <span
                        onClick={
                          header.column.getCanSort()
                            ? header.column.getToggleSortingHandler()
                            : undefined
                        }
                        className={cn(
                          'flex flex-1 items-center gap-1 px-2 py-2',
                          meta?.align === 'right'
                            ? 'justify-end'
                            : 'justify-start',
                          header.column.getCanSort()
                            ? 'cursor-pointer'
                            : 'cursor-default',
                        )}
                      >
                        {flexRender(
                          header.column.columnDef.header,
                          header.getContext(),
                        )}
                        {sortDirection
                          ? SORT_INDICATORS[
                              sortDirection as keyof typeof SORT_INDICATORS
                            ]
                          : null}
                      </span>
                      {header.column.getCanResize() && (
                        <div
                          className="absolute right-0 top-0 h-full w-1 translate-x-1/2 cursor-col-resize hover:bg-action-active"
                          onClick={(event) => event.stopPropagation()}
                          onMouseDown={(event) => {
                            event.stopPropagation()
                            header.getResizeHandler()(event)
                          }}
                          onTouchStart={(event) => {
                            event.stopPropagation()
                            header.getResizeHandler()(event)
                          }}
                        />
                      )}
                    </div>
                  )
                })}
              </div>
            ))}
          </div>
          <div className="relative" style={{ height: totalSize }}>
            {virtualRows.map((virtualRow) => {
              const row = rows[virtualRow.index]
              if (!row) return null

              return (
                <RowComponent
                  key={row.id}
                  row={row}
                  virtualStart={virtualRow.start}
                  virtualSize={virtualRow.size}
                  onShowDetail={onShowDetail}
                />
              )
            })}
          </div>
        </div>
      </div>
    </div>
  )
}
