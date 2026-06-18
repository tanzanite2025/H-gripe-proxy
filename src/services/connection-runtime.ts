import { invoke } from '@tauri-apps/api/core'

export function connectionUsesProxy(
  connection: Pick<IConnectionsItem, 'chains'>,
  proxyName: string,
) {
  return connection.chains.includes(proxyName)
}

export async function getRuntimeConnections() {
  return invoke<IConnections>('get_runtime_connections')
}

export async function closeRuntimeConnection(connectionId: string) {
  await invoke<void>('close_runtime_connection', { connectionId })
}

export async function closeAllRuntimeConnections() {
  await invoke<void>('close_all_runtime_connections')
}

export async function closeConnectionsForProxy(proxyName: string) {
  const { connections } = await getRuntimeConnections()
  const matchingConnections = (connections ?? []).filter((connection) =>
    connectionUsesProxy(connection, proxyName),
  )

  if (matchingConnections.length === 0) {
    return 0
  }

  await Promise.allSettled(
    matchingConnections.map((connection) => closeRuntimeConnection(connection.id)),
  )

  return matchingConnections.length
}
