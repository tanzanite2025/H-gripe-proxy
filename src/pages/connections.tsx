import { useLockFn } from 'ahooks'
import { Trash2, Table, Rows, Columns } from 'lucide-react'
import { useCallback, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  BaseEmpty,
  BasePage,
  BaseSearchBox,
  BaseStyledSelect,
  type SearchState,
  VirtualList,
} from '@/components/base'
import {
  ConnectionDetail,
  ConnectionDetailRef,
} from '@/components/connection/connection-detail'
import { ConnectionItem } from '@/components/connection/connection-item'
import {
  CONNECTION_ORDER_OPTIONS,
  type ConnectionOrderKey,
  filterAndOrderConnections,
  getConnectionViewSpec,
} from '@/components/connection/connection-page-model'
import { ConnectionTable } from '@/components/connection/connection-table'
import {
  Box,
  Button,
  ButtonGroup,
  Fab,
  IconButton,
  SelectMenuItem,
  Tooltip,
  Zoom,
} from '@/components/tailwind'
import { useConnectionData } from '@/hooks/data'
import { useConnectionSetting } from '@/hooks/system'
import { closeAllRuntimeConnections } from '@/services/connection-runtime'
import parseTraffic from '@/utils/format'

const ConnectionsPage = () => {
  const { t } = useTranslation()
  const [match, setMatch] = useState<(input: string) => boolean>(
    () => () => true,
  )
  const [searchText, setSearchText] = useState('')
  const [curOrderOpt, setCurOrderOpt] = useState<ConnectionOrderKey>('default')
  const [connectionsType, setConnectionsType] = useState<'active' | 'closed'>(
    'active',
  )

  const {
    response: { data: connections },
    clearClosedConnections,
  } = useConnectionData()

  const [setting, setSetting] = useConnectionSetting()

  const isTableLayout = setting.layout === 'table'
  const viewSpec = useMemo(
    () => getConnectionViewSpec(connectionsType),
    [connectionsType],
  )

  const [isColumnManagerOpen, setIsColumnManagerOpen] = useState(false)

  const [filterConn] = useMemo(() => {
    const conns =
      (connectionsType === 'active'
        ? connections?.activeConnections
        : connections?.closedConnections) ?? []

    return [
      filterAndOrderConnections({
        connections: conns,
        match,
        orderKey: curOrderOpt,
        searchText,
        viewMode: viewSpec.mode,
      }),
    ]
  }, [connections, connectionsType, curOrderOpt, match, searchText, viewSpec.mode])

  const onCloseAll = useLockFn(closeAllRuntimeConnections)

  const detailRef = useRef<ConnectionDetailRef>(null!)

  const handleSearch = useCallback((
    match: (content: string) => boolean,
    state: SearchState,
  ) => {
    setMatch(() => match)
    setSearchText(state.text)
  }, [])

  const hasTableData = filterConn.length > 0

  return (
    <BasePage
      full
      title={
        <span style={{ whiteSpace: 'nowrap' }}>
          {t('connections.page.title')}
        </span>
      }
      contentStyle={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        borderRadius: '8px',
        minHeight: 0,
      }}
      header={null}
    >
      <Box className="pt-4 mb-2 mx-[10px] select-text sticky top-0 z-[2]">
        <Box className="flex items-center gap-4 mb-2">
          <ButtonGroup className="uds-toolbar mr-1" style={{ flexBasis: 'content' }}>
            <Button
              size="small"
              variant={connectionsType === 'active' ? 'primary' : 'outlined'}
              onClick={() => setConnectionsType('active')}
            >
              {t('connections.components.actions.active')}{' '}
              {connections?.activeConnections.length}
            </Button>
            <Button
              size="small"
              variant={connectionsType === 'closed' ? 'primary' : 'outlined'}
              onClick={() => setConnectionsType('closed')}
            >
              {t('connections.components.actions.closed')}{' '}
              {connections?.closedConnections.length}
            </Button>
          </ButtonGroup>
          {!isTableLayout && viewSpec.showSortControl && (
            <BaseStyledSelect
              value={curOrderOpt}
              onChange={(e) =>
                setCurOrderOpt(e.target.value as ConnectionOrderKey)
              }
            >
              {CONNECTION_ORDER_OPTIONS.map((option) => (
                <SelectMenuItem key={option.id} value={option.id}>
                  <span style={{ fontSize: 14 }}>{t(option.labelKey)}</span>
                </SelectMenuItem>
              ))}
            </BaseStyledSelect>
          )}
          {viewSpec.showTrafficTotals && (
            <Box className="mx-1">
              {t('shared.labels.downloaded')}:{' '}
              {parseTraffic(connections?.downloadTotal)}
            </Box>
          )}
          {viewSpec.showTrafficTotals && (
            <Box className="mx-1">
              {t('shared.labels.uploaded')}:{' '}
              {parseTraffic(connections?.uploadTotal)}
            </Box>
          )}
          <IconButton
            color="inherit"
            size="small"
            onClick={() =>
              setSetting((o) =>
                o?.layout !== 'table'
                  ? { ...o, layout: 'table' }
                  : { ...o, layout: 'list' },
              )
            }
          >
            {isTableLayout ? (
              <span title={t('shared.actions.listView')}>
                <Rows className="h-5 w-5" />
              </span>
            ) : (
              <span title={t('shared.actions.tableView')}>
                <Table className="h-5 w-5" />
              </span>
            )}
          </IconButton>
          {isTableLayout && hasTableData && (
            <Tooltip title={t('connections.components.columnManager.title')}>
              <IconButton
                size="small"
                aria-label={t('connections.components.columnManager.title')}
                onClick={() => setIsColumnManagerOpen(true)}
              >
                <Columns />
              </IconButton>
            </Tooltip>
          )}
          <div className="flex-1" />
          {viewSpec.showCloseAllAction && (
            <Button size="small" variant="primary" onClick={onCloseAll}>
              <span style={{ whiteSpace: 'nowrap' }}>
                {t('shared.actions.closeAll')}
              </span>
            </Button>
          )}
        </Box>
        <Box className="flex items-center">
          <BaseSearchBox onSearch={handleSearch} />
        </Box>
      </Box>

      {!hasTableData ? (
        <BaseEmpty />
      ) : isTableLayout ? (
        <ConnectionTable
          connections={filterConn}
          viewMode={viewSpec.mode}
          onShowDetail={(detail) =>
            detailRef.current?.open(detail, viewSpec.mode)
          }
          columnManagerOpen={isTableLayout && isColumnManagerOpen}
          onCloseColumnManager={() => setIsColumnManagerOpen(false)}
        />
      ) : (
        <VirtualList
          count={filterConn.length}
          estimateSize={56}
          renderItem={(i) => (
            <ConnectionItem
              value={filterConn[i]}
              viewMode={viewSpec.mode}
              onShowDetail={() =>
                detailRef.current?.open(
                  filterConn[i],
                  viewSpec.mode,
                )
              }
            />
          )}
          style={{
            flex: 1,
            borderRadius: '8px',
            WebkitOverflowScrolling: 'touch',
            overscrollBehavior: 'contain',
          }}
        />
      )}
      <ConnectionDetail ref={detailRef} />
      <Zoom
        in={connectionsType === 'closed' && filterConn.length > 0}
        unmountOnExit
      >
        <Fab
          size="medium"
          variant="extended"
          className="absolute right-4"
          style={{ bottom: isTableLayout ? 70 : 16 }}
          color="primary"
          onClick={() => clearClosedConnections()}
        >
          <Trash2 className="h-5 w-5 mr-1" />
          {t('shared.actions.clear')}
        </Fab>
      </Zoom>
    </BasePage>
  )
}

export default ConnectionsPage
