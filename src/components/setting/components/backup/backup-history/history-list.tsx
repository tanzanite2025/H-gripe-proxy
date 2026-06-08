import { Download, RotateCcw, Trash2 } from 'lucide-react'

import {
  Button,
  IconButton,
  List,
  ListItem,
  ListItemText,
  ListSubheader,
  Stack,
  Typography,
} from '@/components/tailwind'

import type { BackupRow } from './types'

interface BackupHistoryListProps {
  rows: BackupRow[]
  isBusy: boolean
  isLocal: boolean
  title: string
  emptyLabel: string
  previousLabel: string
  nextLabel: string
  currentPage: number
  pageCount: number
  onExport: (filename: string) => void
  onDelete: (filename: string) => void
  onRestore: (filename: string) => void
  onPrevPage: () => void
  onNextPage: () => void
}

export function BackupHistoryList({
  rows,
  isBusy,
  isLocal,
  title,
  emptyLabel,
  previousLabel,
  nextLabel,
  currentPage,
  pageCount,
  onExport,
  onDelete,
  onRestore,
  onPrevPage,
  onNextPage,
}: BackupHistoryListProps) {
  return (
    <>
      <List
        disablePadding
        subheader={<ListSubheader disableSticky>{title}</ListSubheader>}
      >
        {rows.length === 0 ? (
          <ListItem>
            <ListItemText primary={emptyLabel || ''} />
          </ListItem>
        ) : (
          rows.map((row) => (
            <ListItem key={`${row.platform}-${row.filename}`} divider>
              <ListItemText
                primary={
                  <Typography variant="body2" className="break-all font-medium">
                    {row.filename}
                  </Typography>
                }
                secondary={
                  <Stack
                    direction="row"
                    spacing={1.5}
                    className="items-center justify-between"
                  >
                    <Typography variant="caption" className="text-secondary">
                      {`${row.platform} / ${row.displayTime}`}
                    </Typography>
                    <Stack direction="row" spacing={0.5} className="items-center">
                      {isLocal && (
                        <IconButton
                          size="small"
                          disabled={isBusy}
                          onClick={() => onExport(row.filename)}
                        >
                          <Download className="h-4 w-4" />
                        </IconButton>
                      )}
                      <IconButton
                        size="small"
                        disabled={isBusy}
                        onClick={() => onDelete(row.filename)}
                      >
                        <Trash2 className="h-4 w-4" />
                      </IconButton>
                      <IconButton
                        size="small"
                        disabled={isBusy}
                        onClick={() => onRestore(row.filename)}
                      >
                        <RotateCcw className="h-4 w-4" />
                      </IconButton>
                    </Stack>
                  </Stack>
                }
              />
            </ListItem>
          ))
        )}
      </List>

      {pageCount > 1 && (
        <Stack direction="row" spacing={1} className="items-center justify-end">
          <Typography variant="caption">
            {currentPage + 1} / {pageCount}
          </Typography>
          <Stack direction="row" spacing={1}>
            <Button
              size="small"
              variant="text"
              disabled={isBusy || currentPage === 0}
              onClick={onPrevPage}
            >
              {previousLabel}
            </Button>
            <Button
              size="small"
              variant="text"
              disabled={isBusy || currentPage >= pageCount - 1}
              onClick={onNextPage}
            >
              {nextLabel}
            </Button>
          </Stack>
        </Stack>
      )}
    </>
  )
}
