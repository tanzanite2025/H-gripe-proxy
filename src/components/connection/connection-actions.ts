type CreateCloseConnectionActionOptions = {
  closed: boolean
  connectionId: string
  onCloseConnection: (connectionId: string) => Promise<void>
}

export const createCloseConnectionAction = ({
  closed,
  connectionId,
  onCloseConnection,
}: CreateCloseConnectionActionOptions) => {
  if (closed) return null

  return {
    onAction: () => onCloseConnection(connectionId),
  }
}
