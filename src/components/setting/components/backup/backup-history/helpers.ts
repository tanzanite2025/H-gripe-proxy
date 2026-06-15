import dayjs from 'dayjs'
import customParseFormat from 'dayjs/plugin/customParseFormat'
import relativeTime from 'dayjs/plugin/relativeTime'

import { BACKUP_DATE_FORMAT, BACKUP_FILENAME_PATTERN } from './constants'
import type { BackupRow } from './types'

dayjs.extend(customParseFormat)
dayjs.extend(relativeTime)

interface BackupRowLabels {
  unknownPlatform: string
  unknownTime: string
}

export function buildBackupRow(
  item: ILocalBackupFile | IWebDavFile,
  labels: BackupRowLabels,
): BackupRow | null {
  const { filename, last_modified: lastModified } = item
  if (!filename.toLowerCase().endsWith('.zip')) {
    return null
  }

  const platform =
    (filename.includes('-') && filename.split('-')[0]) || labels.unknownPlatform
  const matchedTimestamp = filename.match(BACKUP_FILENAME_PATTERN)
  const parsedFromFilename = matchedTimestamp
    ? dayjs(matchedTimestamp[0], BACKUP_DATE_FORMAT, true)
    : null
  const parsedFromModified =
    lastModified && dayjs(lastModified).isValid() ? dayjs(lastModified) : null
  const backupTime = parsedFromFilename?.isValid()
    ? parsedFromFilename
    : parsedFromModified

  return {
    filename,
    platform,
    backupTime: backupTime ?? null,
    displayTime:
      backupTime?.format('YYYY-MM-DD HH:mm') ??
      parsedFromModified?.format('YYYY-MM-DD HH:mm') ??
      labels.unknownTime,
    sortValue:
      backupTime?.valueOf() ??
      parsedFromModified?.valueOf() ??
      Number.NEGATIVE_INFINITY,
  }
}

export function sortBackupRows(rows: BackupRow[]) {
  return rows.sort((left, right) =>
    left.sortValue === right.sortValue
      ? right.filename.localeCompare(left.filename)
      : right.sortValue - left.sortValue,
  )
}
