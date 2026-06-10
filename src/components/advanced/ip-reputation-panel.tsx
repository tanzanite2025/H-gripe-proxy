import { useQuery } from '@tanstack/react-query'
import { useEffect, useState } from 'react'

import {
  getIdentityConsistencyDriftReport,
  getIdentityConsistencyHistory,
  getIdentityConsistencyReport,
} from '@/services/cmds/diagnostics'
import {
  ipReputationCheckIp,
  ipReputationClearCache,
  ipReputationGetCacheEntries,
  ipReputationGetCacheStats,
  ipReputationProbeMetadataProvider,
} from '@/services/ip-reputation/api'
import type {
  IpMetadataProviderHealthReport,
  IpReputation,
  IpReputationConfig,
} from '@/services/ip-reputation/model'

import { IpReputationCacheCard } from './ip-reputation-panel/cache-card'
import { IpReputationConsistencyCard } from './ip-reputation-panel/consistency-card'
import { IpReputationLookupCard } from './ip-reputation-panel/lookup-card'
import { IpReputationSettingsCard } from './ip-reputation-panel/settings-card'

interface Props {
  config: IpReputationConfig
  onChange: (config: IpReputationConfig) => void
}

export function IpReputationPanel({ config, onChange }: Props) {
  const [checkIp, setCheckIp] = useState('')
  const [checking, setChecking] = useState(false)
  const [result, setResult] = useState<IpReputation | null>(null)
  const [cacheEntries, setCacheEntries] = useState<IpReputation[]>([])
  const [cacheStats, setCacheStats] = useState<[number, number] | null>(null)
  const [showCache, setShowCache] = useState(false)
  const [providerProbeIp, setProviderProbeIp] = useState('1.1.1.1')
  const [providerProbeLoading, setProviderProbeLoading] = useState(false)
  const [providerProbeResult, setProviderProbeResult] =
    useState<IpMetadataProviderHealthReport | null>(null)

  useEffect(() => {
    setProviderProbeResult(null)
  }, [config])

  const {
    data: consistencyReport,
    error: consistencyError,
    isFetching: consistencyFetching,
    refetch: refetchConsistencyReport,
  } = useQuery({
    queryKey: ['identity-consistency-report'],
    queryFn: getIdentityConsistencyReport,
    staleTime: 30 * 1000,
    gcTime: 5 * 60 * 1000,
    retry: 1,
  })

  const {
    data: consistencyHistory = [],
    refetch: refetchConsistencyHistory,
  } = useQuery({
    queryKey: ['identity-consistency-history'],
    queryFn: getIdentityConsistencyHistory,
    staleTime: 30 * 1000,
    gcTime: 5 * 60 * 1000,
    retry: 1,
  })

  const {
    data: consistencyDriftReport,
    refetch: refetchConsistencyDriftReport,
  } = useQuery({
    queryKey: ['identity-consistency-drift-report'],
    queryFn: getIdentityConsistencyDriftReport,
    staleTime: 30 * 1000,
    gcTime: 5 * 60 * 1000,
    retry: 1,
  })

  const handleRefreshConsistency = async () => {
    await Promise.all([
      refetchConsistencyReport(),
      refetchConsistencyHistory(),
      refetchConsistencyDriftReport(),
    ])
  }

  const handleCheck = async () => {
    if (!checkIp.trim()) return

    setChecking(true)
    try {
      const reputation = await ipReputationCheckIp(checkIp.trim())
      setResult(reputation)
    } catch {
      setResult(null)
    } finally {
      setChecking(false)
    }
  }

  const handleRefreshCache = async () => {
    const [stats, entries] = await Promise.all([
      ipReputationGetCacheStats(),
      ipReputationGetCacheEntries(),
    ])
    setCacheStats(stats)
    setCacheEntries(entries)
    setShowCache(true)
  }

  const handleClearCache = async () => {
    await ipReputationClearCache()
    setCacheStats(null)
    setCacheEntries([])
  }

  const handleToggleEnabled = (enabled: boolean) => {
    onChange({ ...config, enabled })
  }

  const handleUpdateTtl = (value: string) => {
    const ttl = parseInt(value, 10)
    if (!Number.isNaN(ttl) && ttl > 0) {
      onChange({ ...config, cacheTtl: ttl })
    }
  }

  const handleProbeMetadataProvider = async () => {
    setProviderProbeLoading(true)
    try {
      const report = await ipReputationProbeMetadataProvider(
        config.metadataProvider,
        providerProbeIp.trim() || undefined,
      )
      setProviderProbeResult(report)
    } finally {
      setProviderProbeLoading(false)
    }
  }

  return (
    <div className="space-y-4">
      <IpReputationConsistencyCard
        report={consistencyReport}
        history={consistencyHistory}
        driftReport={consistencyDriftReport}
        isRefreshing={consistencyFetching}
        hasError={Boolean(consistencyError)}
        onRefresh={handleRefreshConsistency}
      />

      <IpReputationSettingsCard
        config={config}
        providerProbeLoading={providerProbeLoading}
        providerProbeIp={providerProbeIp}
        providerProbeResult={providerProbeResult}
        onToggleEnabled={handleToggleEnabled}
        onTtlChange={handleUpdateTtl}
        onRefreshCache={handleRefreshCache}
        onClearCache={handleClearCache}
        onProviderProbeIpChange={setProviderProbeIp}
        onProbeProvider={handleProbeMetadataProvider}
      />

      <IpReputationLookupCard
        enabled={config.enabled}
        checkIp={checkIp}
        checking={checking}
        result={result}
        onCheckIpChange={setCheckIp}
        onCheck={handleCheck}
      />

      <IpReputationCacheCard
        enabled={config.enabled}
        visible={showCache}
        stats={cacheStats}
        entries={cacheEntries}
      />
    </div>
  )
}
