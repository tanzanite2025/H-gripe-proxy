import {
  ArrowUpDown,
  PauseCircle,
  PlayCircle,
  Settings2,
} from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  BaseEmpty,
  BasePage,
  BaseSearchBox,
  type SearchState,
  VirtualList,
  type VirtualListHandle,
} from '@/components/base'
import LogItem from '@/components/log/log-item'
import { LogSettingsPanel } from '@/components/log/log-settings-panel'
import {
  Box,
  Button,
  Card,
  Chip,
  Collapse,
  IconButton,
  Select,
  SelectMenuItem,
} from '@/components/tailwind'
import { useLogData } from '@/hooks/data'
import { useClashLog } from '@/hooks/system'

const LOG_FILTER_LABEL_KEYS: Record<LogFilter, string> = {
  all: 'shared.filters.logLevels.all',
  debug: 'shared.filters.logLevels.debug',
  info: 'shared.filters.logLevels.info',
  warn: 'shared.filters.logLevels.warn',
  err: 'shared.filters.logLevels.error',
}

const LOG_FILTER_CHIP_COLORS: Record<
  LogFilter,
  'default' | 'primary' | 'secondary' | 'error' | 'warning' | 'info' | 'success'
> = {
  all: 'default',
  debug: 'secondary',
  info: 'info',
  warn: 'warning',
  err: 'error',
}

const LogPage = () => {
  const { t } = useTranslation()
  const [clashLog, setClashLog] = useClashLog()
  const enableLog = clashLog.enable
  const logState = clashLog.logFilter
  const logOrder = clashLog.logOrder ?? 'asc'
  const isDescending = logOrder === 'desc'
  const [showSettings, setShowSettings] = useState(true)

  const [match, setMatch] = useState(() => (_: string) => true)
  const [searchState, setSearchState] = useState<SearchState>()
  const {
    response: { data: logData },
    refreshGetClashLog,
  } = useLogData()

  const filterLogs = useMemo(() => {
    if (!logData || logData.length === 0) {
      return []
    }

    // Server-side filtering handles level filtering via query parameters
    // We only need to apply search filtering here
    return logData.filter((data) => {
      // 构建完整的搜索文本，包含时间、类型和内容
      const searchText =
        `${data.time || ''} ${data.type} ${data.payload}`.toLowerCase()

      const matchesSearch = match(searchText)

      return (
        (logState == 'all' ? true : data.type.includes(logState)) &&
        matchesSearch
      )
    })
  }, [logData, logState, match])

  const filteredLogs = useMemo(
    () => (isDescending ? [...filterLogs].reverse() : filterLogs),
    [filterLogs, isDescending],
  )
  const totalLogCount = logData?.length ?? 0

  const virtuosoRef = useRef<VirtualListHandle>(null)

  useEffect(() => {
    if (!isDescending && filteredLogs.length > 0) {
      virtuosoRef.current?.scrollToIndex(filteredLogs.length - 1, {
        behavior: 'smooth',
      })
    }
  }, [filteredLogs.length, isDescending])

  const handleLogLevelChange = (newLevel: LogFilter) => {
    setClashLog((pre) => ({ ...pre!, logFilter: newLevel }))
  }

  const handleToggleLog = async () => {
    setClashLog((pre) => ({ ...pre!, enable: !enableLog }))
  }

  const handleToggleOrder = () => {
    setClashLog((pre) => ({
      ...pre!,
      logOrder: pre!.logOrder === 'desc' ? 'asc' : 'desc',
    }))
  }

  return (
    <BasePage
      full
      title={t('logs.page.title')}
      contentStyle={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        minHeight: 0,
      }}
      header={null}
    >
      <Box className="flex h-full min-h-0 flex-col gap-3 px-[10px] pb-3 pt-3">
        <Card
          variant="outlined"
          className="shrink-0 overflow-hidden border-divider shadow-none"
        >
          <Box className="flex flex-col gap-3 p-4 lg:flex-row lg:items-center lg:justify-between">
            <Box className="flex min-w-0 flex-col gap-3">
              <Box className="flex flex-wrap items-center gap-2">
                <Chip
                  size="small"
                  color={enableLog ? 'success' : 'warning'}
                  label={t(
                    enableLog
                      ? 'shared.statuses.enabled'
                      : 'shared.statuses.disabled',
                  )}
                />
                <Chip
                  size="small"
                  variant="outlined"
                  color={LOG_FILTER_CHIP_COLORS[logState]}
                  label={t(LOG_FILTER_LABEL_KEYS[logState])}
                />
                <Chip
                  size="small"
                  variant="outlined"
                  color="default"
                  label={`${filteredLogs.length} / ${totalLogCount}`}
                />
              </Box>

              <Box className="text-xs leading-5 text-text-secondary">
                {t('settings.sections.clash.form.tooltips.logLevel')}
              </Box>
            </Box>

            <Box className="flex flex-wrap items-center gap-2">
              <IconButton
                title={t(
                  enableLog ? 'shared.actions.pause' : 'shared.actions.resume',
                )}
                aria-label={t(
                  enableLog ? 'shared.actions.pause' : 'shared.actions.resume',
                )}
                size="small"
                color="inherit"
                onClick={handleToggleLog}
              >
                {enableLog ? <PauseCircle /> : <PlayCircle />}
              </IconButton>

              <IconButton
                title={t(
                  isDescending
                    ? 'logs.actions.showAscending'
                    : 'logs.actions.showDescending',
                )}
                aria-label={t(
                  isDescending
                    ? 'logs.actions.showAscending'
                    : 'logs.actions.showDescending',
                )}
                size="small"
                color="inherit"
                onClick={handleToggleOrder}
              >
                <ArrowUpDown
                  className={`h-5 w-5 transition-transform duration-200 ${
                    isDescending ? 'scale-y-[-1]' : ''
                  }`}
                />
              </IconButton>

              <Button
                size="small"
                variant={showSettings ? 'primary' : 'outlined'}
                color={showSettings ? 'primary' : 'inherit'}
                startIcon={<Settings2 className="h-4 w-4" />}
                onClick={() => setShowSettings((current) => !current)}
              >
                {t('settings.page.title')}
              </Button>

              <Button
                size="small"
                variant="primary"
                onClick={() => {
                  refreshGetClashLog(true)
                }}
              >
                {t('shared.actions.clear')}
              </Button>
            </Box>
          </Box>
        </Card>

        <Collapse in={showSettings} className="shrink-0">
          <LogSettingsPanel />
        </Collapse>

        <Card
          variant="outlined"
          className="flex min-h-0 flex-1 flex-col overflow-hidden border-divider shadow-none"
        >
          <Box className="shrink-0 border-b border-divider px-4 py-3">
            <Box className="flex flex-col gap-3 xl:flex-row xl:items-center">
              <Box className="w-full md:w-[160px] xl:w-[180px]">
                <Select
                  size="small"
                  value={logState}
                  onChange={(e) =>
                    handleLogLevelChange(e.target.value as LogFilter)
                  }
                >
                  <SelectMenuItem value="all">
                    {t('shared.filters.logLevels.all')}
                  </SelectMenuItem>
                  <SelectMenuItem value="debug">
                    {t('shared.filters.logLevels.debug')}
                  </SelectMenuItem>
                  <SelectMenuItem value="info">
                    {t('shared.filters.logLevels.info')}
                  </SelectMenuItem>
                  <SelectMenuItem value="warn">
                    {t('shared.filters.logLevels.warn')}
                  </SelectMenuItem>
                  <SelectMenuItem value="err">
                    {t('shared.filters.logLevels.error')}
                  </SelectMenuItem>
                </Select>
              </Box>

              <Box className="min-w-0 flex-1">
                <BaseSearchBox
                  onSearch={(matcher, state) => {
                    setMatch(() => matcher)
                    setSearchState(state)
                  }}
                />
              </Box>
            </Box>
          </Box>

          <Box className="flex min-h-0 flex-1 flex-col">
            {filteredLogs.length > 0 ? (
              <VirtualList
                ref={virtuosoRef}
                count={filteredLogs.length}
                estimateSize={64}
                renderItem={(i) => (
                  <LogItem value={filteredLogs[i]} searchState={searchState} />
                )}
                style={{ flex: 1, minHeight: 0 }}
              />
            ) : (
              <BaseEmpty />
            )}
          </Box>
        </Card>
      </Box>
    </BasePage>
  )
}

export default LogPage
