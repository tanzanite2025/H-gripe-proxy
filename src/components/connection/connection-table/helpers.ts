import { buildConnectionViewModel } from '../connection-view-model'

import type { ColumnField } from './types'

export const reconcileColumnOrder = (
  storedOrder: string[],
  baseFields: string[],
) => {
  const filtered = storedOrder.filter((field) => baseFields.includes(field))
  const missing = baseFields.filter((field) => !filtered.includes(field))
  return [...filtered, ...missing]
}

export const getConnectionCellValue = (
  field: ColumnField,
  connection: IConnectionsItem,
) => {
  const viewModel = buildConnectionViewModel(connection)

  switch (field) {
    case 'host':
      return viewModel.host
    case 'download':
      return connection.download
    case 'upload':
      return connection.upload
    case 'dlSpeed':
      return connection.curDownload
    case 'ulSpeed':
      return connection.curUpload
    case 'chains':
      return viewModel.chains
    case 'rule':
      return viewModel.rule
    case 'process':
      return viewModel.process
    case 'time':
      return connection.start
    case 'source':
      return viewModel.source
    case 'remoteDestination':
      return viewModel.remoteDestination
    case 'type':
      return viewModel.type
    case 'actions':
      return ''
    default:
      return ''
  }
}
