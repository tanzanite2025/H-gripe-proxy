import { useLockFn } from 'ahooks'
import yaml from 'js-yaml'
import { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseSearchBox } from '@/components/base'
import {
  Button,
  Checkbox,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  ListItemButton,
  ListItemText,
} from '@/components/tailwind'
import { useProfiles } from '@/hooks/data'
import { useProxiesData } from '@/providers/app-data-context'
import {
  enhanceProfiles,
  readProfileFile,
  saveProfileFile,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import {
  categorizeProxyGroup,
  isProxyGroupItem,
} from '@/services/proxy-display'
import { cn } from '@/utils/cn'

import {
  buildGroupsYaml,
  parseGroupsYaml,
} from '../profile/groups-editor-viewer/utils/group-helpers'

type EditableStrategyGroupState = {
  baseGroup: IProxyGroupConfig
  originExists: boolean
}

type CandidateOption = {
  name: string
  type: string
  provider?: string
  isGroup: boolean
}

const BUILTIN_PROXY_NAMES = new Set([
  'DIRECT',
  'REJECT',
  'REJECT-DROP',
  'PASS',
])

const RUNTIME_GROUP_TYPE_MAP: Record<string, IProxyGroupConfig['type']> = {
  Selector: 'select',
  URLTest: 'url-test',
  LoadBalance: 'load-balance',
  Fallback: 'fallback',
  Relay: 'relay',
}

const normalizeNames = (names: Array<string | null | undefined>) =>
  Array.from(
    new Set(
      names
        .map((name) => name?.trim() || '')
        .filter((name) => name.length > 0),
    ),
  )

const cloneGroupConfig = (group: IProxyGroupConfig): IProxyGroupConfig => ({
  ...group,
  proxies: Array.isArray(group.proxies) ? [...group.proxies] : undefined,
  use: Array.isArray(group.use) ? [...group.use] : undefined,
})

const findLastGroupByName = (
  list: IProxyGroupConfig[],
  name: string,
): IProxyGroupConfig | undefined => {
  for (let index = list.length - 1; index >= 0; index -= 1) {
    const item = list[index]
    if (item?.name === name) {
      return item
    }
  }

  return undefined
}

const buildFallbackGroupConfig = (
  group: IProxyGroupItem,
): IProxyGroupConfig => ({
  name: group.name,
  type: RUNTIME_GROUP_TYPE_MAP[group.type] || 'url-test',
  proxies: [],
  url: group.testUrl,
  hidden: group.hidden,
  icon: group.icon,
  'interface-name': '',
})

const loadEditableStrategyGroup = async (
  group: IProxyGroupItem,
  profileUid: string,
  property: string,
) => {
  const [profileData, groupsData] = await Promise.all([
    readProfileFile(profileUid),
    readProfileFile(property),
  ])

  const profileObject = yaml.load(profileData) as
    | { 'proxy-groups'?: IProxyGroupConfig[] }
    | null
  const sequence = parseGroupsYaml(groupsData)
  const originGroups = profileObject?.['proxy-groups'] || []
  const originGroup = originGroups.find((item) => item?.name === group.name)
  const overrideGroup =
    findLastGroupByName(sequence.append, group.name) ||
    findLastGroupByName(sequence.prepend, group.name)
  const baseGroup = cloneGroupConfig(
    overrideGroup || originGroup || buildFallbackGroupConfig(group),
  )

  const selectedNames = normalizeNames(
    Array.isArray(overrideGroup?.proxies) ? overrideGroup.proxies : [],
  )

  return {
    sequence,
    state: {
      baseGroup,
      originExists: Boolean(originGroup),
    } satisfies EditableStrategyGroupState,
    selectedNames,
  }
}

interface Props {
  open: boolean
  group: IProxyGroupItem | null
  onClose: () => void
  onSaved?: () => Promise<void> | void
}

export function StrategyPoolEditorDialog({
  open,
  group,
  onClose,
  onSaved,
}: Props) {
  const { t } = useTranslation()
  const { current, mutateProfiles } = useProfiles()
  const { proxies: proxiesData } = useProxiesData()

  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const [searchText, setSearchText] = useState('')
  const [selectedNames, setSelectedNames] = useState<string[]>([])
  const [editableState, setEditableState] =
    useState<EditableStrategyGroupState | null>(null)

  useEffect(() => {
    if (!open || !group) {
      setLoading(false)
      setSaving(false)
      setSearchText('')
      setSelectedNames([])
      setEditableState(null)
      return
    }

    const property = current?.option?.groups?.trim() || ''
    const profileUid = current?.uid?.trim() || ''

    if (!property || !profileUid) {
      setEditableState(null)
      setSelectedNames([])
      return
    }

    let cancelled = false
    setLoading(true)
    setSearchText('')

    void loadEditableStrategyGroup(group, profileUid, property)
      .then((result) => {
        if (cancelled) return
        setEditableState(result.state)
        setSelectedNames(result.selectedNames)
      })
      .catch((error) => {
        if (cancelled) return
        setEditableState(null)
        setSelectedNames([])
        showNotice.error(error)
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [current?.option?.groups, current?.uid, group, open])

  const candidateOptions = useMemo(() => {
    const records = (proxiesData?.records || {}) as Record<string, IProxyItem>
    const selectedOrder = new Map(
      selectedNames.map((name, index) => [name, index]),
    )
    const options = new Map<string, CandidateOption>()

    ;(Object.values(records) as IProxyItem[]).forEach((record) => {
      if (!record?.name) return
      if (BUILTIN_PROXY_NAMES.has(record.name)) return
      if (categorizeProxyGroup(record) === 'auxiliary') return
      if (isProxyGroupItem(record)) return

      options.set(record.name, {
        name: record.name,
        type: record.type,
        provider: record.provider,
        isGroup: false,
      })
    })

    selectedNames.forEach((name) => {
      if (options.has(name)) return

      const record = records[name]
      if (!record) {
        options.set(name, {
          name,
          type: 'Unknown',
          isGroup: false,
        })
        return
      }

      options.set(name, {
        name,
        type: record.type,
        provider: record.provider,
        isGroup: isProxyGroupItem(record),
      })
    })

    return Array.from(options.values())
      .filter((option) => {
        const keyword = searchText.trim().toLowerCase()
        if (!keyword) return true

        return [option.name, option.provider, option.type]
          .filter(Boolean)
          .some((value) => value!.toLowerCase().includes(keyword))
      })
      .sort((left, right) => {
        const leftIndex = selectedOrder.get(left.name)
        const rightIndex = selectedOrder.get(right.name)

        if (leftIndex != null && rightIndex != null) {
          return leftIndex - rightIndex
        }

        if (leftIndex != null) return -1
        if (rightIndex != null) return 1

        return left.name.localeCompare(right.name, 'zh-CN', {
          numeric: true,
          sensitivity: 'base',
        })
      })
  }, [proxiesData?.records, searchText, selectedNames])

  const toggleSelected = (name: string, checked?: boolean) => {
    setSelectedNames((prev) => {
      const exists = prev.includes(name)
      const nextChecked = checked ?? !exists

      if (nextChecked) {
        return exists ? prev : [...prev, name]
      }

      return prev.filter((item) => item !== name)
    })
  }

  const handleSave = useLockFn(async () => {
    if (!group) return

    const property = current?.option?.groups?.trim() || ''
    const profileUid = current?.uid?.trim() || ''

    if (!property || !profileUid) {
      showNotice.error('当前配置缺少 groups 覆盖文件，无法保存策略池成员。')
      return
    }

    if (selectedNames.length === 0) {
      showNotice.error('策略池至少保留一个成员。')
      return
    }

    setSaving(true)

    try {
      const { sequence, state } = await loadEditableStrategyGroup(
        group,
        profileUid,
        property,
      )
      const nextGroup = cloneGroupConfig(state.baseGroup)

      nextGroup.proxies = [...selectedNames]
      delete nextGroup.use
      delete nextGroup['include-all']
      delete nextGroup['include-all-proxies']
      delete nextGroup['include-all-providers']
      delete nextGroup.filter
      delete nextGroup['exclude-filter']
      delete nextGroup['exclude-type']

      const nextPrepend = (sequence.prepend as IProxyGroupConfig[]).filter(
        (item) => item?.name !== group.name,
      )
      const nextAppend = (sequence.append as IProxyGroupConfig[]).filter(
        (item) => item?.name !== group.name,
      )
      const nextDelete = (sequence.delete as string[]).filter(
        (name) => name !== group.name,
      )

      if (state.originExists) {
        nextDelete.push(group.name)
      }

      const nextYaml = buildGroupsYaml(
        nextPrepend,
        [...nextAppend, nextGroup],
        normalizeNames(nextDelete),
      )

      if (!(await saveProfileFile(property, nextYaml))) {
        throw new Error('策略池成员保存失败。')
      }

      if (!(await enhanceProfiles())) {
        throw new Error('策略池成员已写入，但当前配置增强失败。')
      }

      await Promise.all([mutateProfiles(), onSaved?.()])
      showNotice.success('shared.feedback.notifications.saved')
      onClose()
    } catch (error) {
      showNotice.error(error)
    } finally {
      setSaving(false)
    }
  })

  if (!group) {
    return null
  }

  const selectedNameSet = new Set(selectedNames)
  const canSave = !loading && !saving && selectedNames.length > 0

  return (
    <Dialog
      open={open}
      onClose={onClose}
      showCloseButton
      maxWidth="md"
      fullWidth
      slotProps={{ paper: { className: 'max-h-[88vh]' } }}
    >
      <DialogTitle className="pb-3">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="text-base font-semibold text-text-primary">
              {group.name}
            </div>
            <div className="mt-1 text-xs text-text-secondary">
              这里不再自动回填运行时成员，只有你手动勾选并保存的节点才算入池。
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Chip size="small" variant="outlined" label={group.type} />
            <Chip
              size="small"
              variant="outlined"
              color="primary"
              label={`已选 ${selectedNames.length}`}
            />
            <Chip
              size="small"
              variant="outlined"
              label={`可选 ${candidateOptions.length}`}
            />
          </div>
        </div>
      </DialogTitle>

      <DialogContent className="space-y-3">
        <BaseSearchBox
          value={searchText}
          onSearch={(_, state) => setSearchText(state.text)}
        />

        <div className="rounded-2xl border border-white/8 bg-white/5 p-2">
          {loading ? (
            <div className="px-4 py-10 text-center text-sm text-text-secondary">
              正在读取当前策略池配置...
            </div>
          ) : !editableState ? (
            <div className="px-4 py-10 text-center text-sm text-red-400">
              读取策略池配置失败，请确认当前配置已加载完成。
            </div>
          ) : candidateOptions.length === 0 ? (
            <div className="px-4 py-10 text-center text-sm text-text-secondary">
              没有匹配到可加入的节点。
            </div>
          ) : (
            <div className="max-h-[50vh] space-y-2 overflow-y-auto pr-1">
              {candidateOptions.map((option) => {
                const checked = selectedNameSet.has(option.name)

                return (
                  <ListItemButton
                    key={option.name}
                    selected={checked}
                    className={cn(
                      'rounded-xl border border-white/8 px-3 py-2.5',
                      checked
                        ? 'bg-primary/10'
                        : 'bg-black/10 hover:bg-white/8',
                    )}
                    onClick={() => toggleSelected(option.name)}
                  >
                    <div
                      className="mr-3"
                      onClick={(event) => event.stopPropagation()}
                    >
                      <Checkbox
                        checked={checked}
                        onChange={(_, nextChecked) =>
                          toggleSelected(option.name, nextChecked)
                        }
                      />
                    </div>

                    <ListItemText
                      title={option.name}
                      primary={
                        <div className="flex min-w-0 items-center gap-2">
                          <span className="truncate text-sm font-semibold text-text-primary">
                            {option.name}
                          </span>
                          {option.provider ? (
                            <span className="rounded border border-white/10 px-1.5 py-0.5 text-[10px] text-text-secondary">
                              {option.provider}
                            </span>
                          ) : null}
                          {option.isGroup ? (
                            <span className="rounded border border-amber-500/30 px-1.5 py-0.5 text-[10px] text-amber-300">
                              组
                            </span>
                          ) : null}
                        </div>
                      }
                      secondary={
                        <span className="text-[11px] uppercase tracking-wide text-text-secondary">
                          {option.type}
                        </span>
                      }
                    />
                  </ListItemButton>
                )
              })}
            </div>
          )}
        </div>
      </DialogContent>

      <DialogActions>
        <Button variant="outlined" onClick={onClose} disabled={saving}>
          {t('shared.actions.cancel')}
        </Button>
        <Button onClick={handleSave} loading={saving} disabled={!canSave}>
          {t('shared.actions.save')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
