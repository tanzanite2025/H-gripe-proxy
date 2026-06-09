type ProxyChainDelayColor = 'success' | 'warning' | 'error'

interface ProxyChainItemStateOptions {
  isFirst: boolean
  isLast: boolean
  entryLabel: string
  exitLabel: string
  delay?: number
}

export const getProxyChainRoleLabel = ({
  isFirst,
  isLast,
  entryLabel,
  exitLabel,
}: Omit<ProxyChainItemStateOptions, 'delay'>) => {
  if (isFirst) return entryLabel
  if (isLast) return exitLabel
  return undefined
}

export const getProxyChainBorderClass = ({
  isFirst,
  isLast,
}: Pick<ProxyChainItemStateOptions, 'isFirst' | 'isLast'>) => {
  if (isFirst) return 'border-2 border-green-500'
  if (isLast) return 'border-2 border-orange-500'
  return 'border border-divider'
}

export const getProxyChainRoleChipClass = ({
  isFirst,
}: Pick<ProxyChainItemStateOptions, 'isFirst'>) =>
  `mr-2 font-bold text-white ${isFirst ? 'bg-green-500' : 'bg-orange-500'}`

export const getProxyChainDelayColor = (
  delay: number | undefined,
): ProxyChainDelayColor | undefined => {
  if (delay === undefined) {
    return undefined
  }

  if (delay > 0 && delay < 200) {
    return 'success'
  }

  if (delay > 0 && delay < 800) {
    return 'warning'
  }

  return 'error'
}
