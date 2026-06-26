import { FlaskConical } from 'lucide-react'
import { useMemo, useState } from 'react'

import { EnhancedCard } from '@/components/home/enhanced-card'
import { StrategyPoolEditorDialogView } from '@/components/proxy/strategy-pool-editor/strategy-pool-editor-dialog-view'
import type { CandidateOption } from '@/components/proxy/strategy-pool-editor/types'
import {
  buildNextStrategyPoolName,
  createStrategyPoolGroupRef,
} from '@/components/proxy/strategy-pools/strategy-pool-rules'
import { StrategyPoolsSection } from '@/components/proxy/strategy-pools/strategy-pools-section'
import type {
  ManagedStrategyPool,
  StrategyPoolGroupRef,
} from '@/components/proxy/strategy-pools/types'

const FAKE_CANDIDATES: CandidateOption[] = [
  {
    name: '美国01',
    type: 'VMess',
    provider: 'demo-subscription',
    isGroup: false,
  },
  {
    name: '日本01',
    type: 'Trojan',
    provider: 'demo-subscription',
    isGroup: false,
  },
  {
    name: '新加坡01',
    type: 'Shadowsocks',
    provider: 'demo-subscription',
    isGroup: false,
  },
  {
    name: '香港01',
    type: 'Hysteria2',
    provider: 'demo-subscription',
    isGroup: false,
  },
  {
    name: '台湾01',
    type: 'VMess',
    provider: 'demo-subscription',
    isGroup: false,
  },
  {
    name: '德国01',
    type: 'Trojan',
    provider: 'demo-subscription',
    isGroup: false,
  },
]

const INITIAL_MEMBERS: Record<string, string[]> = {
  策略池1: ['美国01', '日本01'],
}

const INITIAL_POOLS: ManagedStrategyPool[] = [
  {
    currentProxyName: '美国01',
    groupRef: createStrategyPoolGroupRef({ name: '策略池1' }),
    memberCount: 2,
    runtimeLoaded: true,
  },
]

export function StrategyPoolRegressionCard() {
  const [editingGroup, setEditingGroup] =
    useState<StrategyPoolGroupRef | null>(null)
  const [membersByGroup, setMembersByGroup] =
    useState<Record<string, string[]>>(INITIAL_MEMBERS)
  const [pools, setPools] = useState<ManagedStrategyPool[]>(INITIAL_POOLS)
  const [searchText, setSearchText] = useState('')
  const [selectedNames, setSelectedNames] = useState<string[]>([])
  const [saving, setSaving] = useState(false)

  const selectedNameSet = useMemo(
    () => new Set(selectedNames),
    [selectedNames],
  )

  const candidateOptions = useMemo(() => {
    const keyword = searchText.trim().toLowerCase()

    return FAKE_CANDIDATES.filter((option) => {
      if (!keyword) return true

      return [option.name, option.provider, option.type]
        .filter(Boolean)
        .some((value) => value!.toLowerCase().includes(keyword))
    })
  }, [searchText])

  const openEditor = (group: StrategyPoolGroupRef) => {
    setEditingGroup(group)
    setSearchText('')
    setSelectedNames(membersByGroup[group.name] || [])
  }

  const handleCreate = () => {
    const nextName = buildNextStrategyPoolName(
      pools.map((pool) => pool.groupRef.name),
    )

    openEditor(createStrategyPoolGroupRef({ name: nextName }))
  }

  const handleToggleSelected = (name: string, checked?: boolean) => {
    setSelectedNames((prev) => {
      const exists = prev.includes(name)
      const nextChecked = checked ?? !exists

      if (nextChecked) {
        return exists ? prev : [...prev, name]
      }

      return prev.filter((item) => item !== name)
    })
  }

  const handleClose = () => {
    setEditingGroup(null)
    setSearchText('')
    setSelectedNames([])
  }

  const handleSave = async () => {
    if (!editingGroup || selectedNames.length === 0) return

    setSaving(true)

    try {
      await new Promise((resolve) => window.setTimeout(resolve, 150))

      const nextMembers = [...selectedNames]
      const nextPool: ManagedStrategyPool = {
        currentProxyName: nextMembers[0] || '未选择',
        groupRef: editingGroup,
        memberCount: nextMembers.length,
        runtimeLoaded: true,
      }

      setMembersByGroup((prev) => ({
        ...prev,
        [editingGroup.name]: nextMembers,
      }))

      setPools((prev) => {
        const exists = prev.some(
          (pool) => pool.groupRef.name === editingGroup.name,
        )

        const next = exists
          ? prev.map((pool) =>
              pool.groupRef.name === editingGroup.name ? nextPool : pool,
            )
          : [...prev, nextPool]

        return next.sort((left, right) =>
          left.groupRef.name.localeCompare(right.groupRef.name, 'zh-CN', {
            numeric: true,
            sensitivity: 'base',
          }),
        )
      })

      handleClose()
    } finally {
      setSaving(false)
    }
  }

  return (
    <>
      <EnhancedCard
        title="策略池本地回归"
        icon={<FlaskConical className="h-4 w-4" />}
        iconColor="info"
      >
        <div className="mb-3 text-xs leading-6 text-text-secondary">
          这个区域完全走本地内存状态，不依赖 Tauri、内核或真实订阅，
          专门用来回归验证策略池的创建、编辑、搜索和保存链路。
        </div>

        <StrategyPoolsSection
          className="mx-0 mb-0 border-white/8 bg-black/10"
          configReady={true}
          pools={pools}
          onCreate={handleCreate}
          onEdit={openEditor}
        />
      </EnhancedCard>

      <StrategyPoolEditorDialogView
        open={Boolean(editingGroup)}
        group={editingGroup}
        candidateOptions={candidateOptions}
        canSave={!saving && selectedNames.length > 0}
        loadWarning=""
        loading={false}
        onClose={handleClose}
        onSave={handleSave}
        onSearchTextChange={setSearchText}
        onToggleSelected={handleToggleSelected}
        saving={saving}
        searchText={searchText}
        selectedNames={selectedNames}
        selectedNameSet={selectedNameSet}
      />
    </>
  )
}
