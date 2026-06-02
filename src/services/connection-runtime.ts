import {
  closeConnection,
  getConnections,
} from 'tauri-plugin-mihomo-api'

export function connectionUsesProxy(
  connection: Pick<IConnectionsItem, 'chains'>,
  proxyName: string,
) {
  return connection.chains.includes(proxyName)
}

export async function closeConnectionsForProxy(proxyName: string) {
  const { connections } = await getConnections()
  const matchingConnections = (connections ?? []).filter((connection) =>
    connectionUsesProxy(connection, proxyName),
  )

  if (matchingConnections.length === 0) {
    return 0
  }

  await Promise.allSettled(
    matchingConnections.map((connection) => closeConnection(connection.id)),
  )

  return matchingConnections.length
}
