import { useEffect, useRef } from 'react'

export function useProxyChainLengthDirtyMarker(
  proxyChainLength: number,
  onMarkUnsavedChanges?: () => void,
) {
  const chainLengthRef = useRef(proxyChainLength)

  useEffect(() => {
    if (
      chainLengthRef.current !== proxyChainLength &&
      chainLengthRef.current !== 0
    ) {
      onMarkUnsavedChanges?.()
    }

    chainLengthRef.current = proxyChainLength
  }, [onMarkUnsavedChanges, proxyChainLength])
}
