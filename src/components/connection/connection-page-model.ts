export type ConnectionOrderKey =
  | 'default'
  | 'uploadSpeed'
  | 'downloadSpeed'

export type ConnectionViewMode = 'active' | 'closed'

export type ConnectionTableField =
  | 'host'
  | 'download'
  | 'upload'
  | 'dlSpeed'
  | 'ulSpeed'
  | 'chains'
  | 'rule'
  | 'process'
  | 'time'
  | 'source'
  | 'remoteDestination'
  | 'type'

export type ConnectionListMetaField =
  | 'network'
  | 'type'
  | 'process'
  | 'chains'
  | 'time'
  | 'speed'
  | 'rule'
  | 'download'
  | 'upload'

export type ConnectionDetailField =
  | 'host'
  | 'download'
  | 'upload'
  | 'dlSpeed'
  | 'ulSpeed'
  | 'chains'
  | 'rule'
  | 'process'
  | 'time'
  | 'source'
  | 'destination'
  | 'destinationPort'
  | 'type'

export interface ConnectionViewSpec {
  mode: ConnectionViewMode
  showTrafficTotals: boolean
  showSortControl: boolean
  showCloseAllAction: boolean
  tableFields: ConnectionTableField[]
  listMetaFields: ConnectionListMetaField[]
  detailFields: ConnectionDetailField[]
}

export const CONNECTION_ORDER_OPTIONS: ReadonlyArray<{
  id: ConnectionOrderKey
  labelKey: string
}> = [
  {
    id: 'default',
    labelKey: 'connections.components.order.default',
  },
  {
    id: 'uploadSpeed',
    labelKey: 'connections.components.order.uploadSpeed',
  },
  {
    id: 'downloadSpeed',
    labelKey: 'connections.components.order.downloadSpeed',
  },
]

type FilterAndOrderConnectionsOptions = {
  connections: IConnectionsItem[]
  match: (input: string) => boolean
  orderKey: ConnectionOrderKey
  searchText: string
  viewMode: ConnectionViewMode
}

const CONNECTION_VIEW_SPECS: Record<ConnectionViewMode, ConnectionViewSpec> = {
  active: {
    mode: 'active',
    showTrafficTotals: true,
    showSortControl: true,
    showCloseAllAction: true,
    tableFields: [
      'host',
      'download',
      'upload',
      'dlSpeed',
      'ulSpeed',
      'chains',
      'rule',
      'process',
      'time',
      'source',
      'remoteDestination',
      'type',
    ],
    listMetaFields: ['network', 'type', 'process', 'chains', 'time', 'speed'],
    detailFields: [
      'host',
      'download',
      'upload',
      'dlSpeed',
      'ulSpeed',
      'chains',
      'rule',
      'process',
      'time',
      'source',
      'destination',
      'destinationPort',
      'type',
    ],
  },
  closed: {
    mode: 'closed',
    showTrafficTotals: false,
    showSortControl: false,
    showCloseAllAction: false,
    tableFields: ['host', 'download', 'upload', 'chains', 'rule', 'process'],
    listMetaFields: ['process', 'rule', 'chains', 'download', 'upload'],
    detailFields: [
      'host',
      'download',
      'upload',
      'chains',
      'rule',
      'process',
      'destination',
      'type',
    ],
  },
}

const sortByStartDesc = (left: IConnectionsItem, right: IConnectionsItem) =>
  new Date(right.start || '0').getTime() - new Date(left.start || '0').getTime()

const sortByUploadSpeed = (left: IConnectionsItem, right: IConnectionsItem) =>
  (right.curUpload ?? 0) - (left.curUpload ?? 0)

const sortByDownloadSpeed = (left: IConnectionsItem, right: IConnectionsItem) =>
  (right.curDownload ?? 0) - (left.curDownload ?? 0)

const orderConnectionComparators: Record<
  ConnectionOrderKey,
  (left: IConnectionsItem, right: IConnectionsItem) => number
> = {
  default: sortByStartDesc,
  uploadSpeed: sortByUploadSpeed,
  downloadSpeed: sortByDownloadSpeed,
}

export const filterAndOrderConnections = ({
  connections,
  match,
  orderKey,
  searchText,
  viewMode,
}: FilterAndOrderConnectionsOptions): IConnectionsItem[] => {
  const filteredConnections = connections.filter((connection) => {
    const { host, destinationIP, process } = connection.metadata
    return (
      match(host || '') || match(destinationIP || '') || match(process || '')
    )
  })

  const effectiveOrderKey =
    viewMode === 'closed' || searchText.trim() ? 'default' : orderKey
  const comparator = orderConnectionComparators[effectiveOrderKey]

  return [...filteredConnections].sort((left, right) => {
    const result = comparator(left, right)
    return result !== 0 ? result : sortByStartDesc(left, right)
  })
}

export const getConnectionViewSpec = (
  viewMode: ConnectionViewMode,
): ConnectionViewSpec => CONNECTION_VIEW_SPECS[viewMode]
