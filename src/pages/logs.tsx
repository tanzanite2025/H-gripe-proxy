import { PlayCircle, PauseCircle, ArrowUpDown } from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  BaseEmpty,
  BasePage,
  BaseSearchBox,
  BaseStyledSelect,
  type SearchState,
  VirtualList,
  type VirtualListHandle,
} from '@/components/base'
import LogItem from '@/components/log/log-item'
import { LogSettingsPanel } from '@/components/log/log-settings-panel'
import { Box, Button, IconButton, SelectMenuItem } from '@/components/tailwind'
import { useLogData } from '@/hooks/data'
import { useClashLog } from '@/hooks/system'

const LogPage = () => {
  const { t } = useTranslation()
  const [clashLog, setClashLog] = useClashLog()
  const enableLog = clashLog.enable
  const logState = clashLog.logFilter
  const logOrder = clashLog.logOrder ?? 'asc'
  const isDescending = logOrder === 'desc'

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
        overflow: 'auto',
      }}
      header={
        <Box className="flex items-center gap-2">
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
            {enableLog ? (
              <PauseCircle />
            ) : (
              <PlayCircle />
            )}
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
              className={`h-5 w-5 transition-transform duration-200 ${isDescending ? 'scale-y-[-1]' : ''}`}
            />
          </IconButton>

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
      }
    >
      <LogSettingsPanel />

      <Box
        className="mb-2 mx-[10px] flex items-center gap-2"
      >
        <BaseStyledSelect
          value={logState}
          onChange={(e) => handleLogLevelChange(e.target.value as LogFilter)}
        >
          <SelectMenuItem value="all">{t('shared.filters.logLevels.all')}</SelectMenuItem>
          <SelectMenuItem value="debug">
            {t('shared.filters.logLevels.debug')}
          </SelectMenuItem>
          <SelectMenuItem value="info">{t('shared.filters.logLevels.info')}</SelectMenuItem>
          <SelectMenuItem value="warn">{t('shared.filters.logLevels.warn')}</SelectMenuItem>
          <SelectMenuItem value="err">{t('shared.filters.logLevels.error')}</SelectMenuItem>
        </BaseStyledSelect>
        <BaseSearchBox
          onSearch={(matcher, state) => {
            setMatch(() => matcher)
            setSearchState(state)
          }}
        />
      </Box>

      {filteredLogs.length > 0 ? (
        <VirtualList
          ref={virtuosoRef}
          count={filteredLogs.length}
          estimateSize={50}
          renderItem={(i) => (
            <LogItem value={filteredLogs[i]} searchState={searchState} />
          )}
          style={{ flex: 1 }}
        />
      ) : (
        <BaseEmpty />
      )}
    </BasePage>
  )
}

export default LogPage
