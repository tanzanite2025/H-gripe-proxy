export interface ConnectionViewModel {
  title: string
  host: string
  destination: string
  remoteDestination: string
  destinationPort: string
  source: string
  chains: string
  hasChains: boolean
  rule: string
  ruleLabel: string
  process: string
  processLabel: string
  network: string
  typeLabel: string
  type: string
}

export const buildConnectionViewModel = (
  connection: IConnectionsItem,
): ConnectionViewModel => {
  const { metadata } = connection
  const host = metadata.host || metadata.remoteDestination
  const destination = metadata.destinationIP || metadata.remoteDestination || ''
  const processLabel = metadata.process || ''
  const ruleLabel = connection.rule || ''
  const chains = [...connection.chains].reverse().join(' / ')

  return {
    title: metadata.host || destination,
    host: host ? `${host}:${metadata.destinationPort}` : '',
    destination,
    destinationPort: `${metadata.destinationPort}`,
    remoteDestination: destination
      ? `${destination}:${metadata.destinationPort}`
      : '',
    source: `${metadata.sourceIP}:${metadata.sourcePort}`,
    chains,
    hasChains: connection.chains.length > 0,
    rule: connection.rulePayload
      ? `${connection.rule}(${connection.rulePayload})`
      : connection.rule,
    ruleLabel,
    process: metadata.process
      ? metadata.processPath
        ? `${metadata.process}(${metadata.processPath})`
        : metadata.process
      : metadata.processPath || '',
    processLabel,
    network: metadata.network,
    typeLabel: metadata.type,
    type: `${metadata.type}(${metadata.network})`,
  }
}
