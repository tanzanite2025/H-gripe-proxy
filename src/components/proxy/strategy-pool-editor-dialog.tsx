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
  isBuiltinPolicyName,
  isHiddenProxyName,
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

type GroupSequence = {
  prepend: IProxyGroupConfig[]
  append: IProxyGroupConfig[]
  delete: string[]
}

type StrategyGroupLoadWarning =
  | 'profileNotReady'
  | 'profileReadFailed'
  | 'groupsReadFailed'

type EditableStrategyGroupLoadResult = {
  sequence: GroupSequence
  state: EditableStrategyGroupState
  selectedNames: string[]
  warnings: StrategyGroupLoadWarning[]
}

type CandidateOption = {
  name: string
  type: string
  provider?: string
  isGroup: boolean
}

const RUNTIME_GROUP_TYPE_MAP: Record<string, IProxyGroupConfig['type']> = {
  Selector: 'select',
  URLTest: 'url-test',
  LoadBalance: 'load-balance',
  Fallback: 'fallback',
  Relay: 'relay',
}

const EMPTY_GROUP_SEQUENCE: GroupSequence = {
  prepend: [],
  append: [],
  delete: [],
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
  profileUid?: string,
  property?: string,
): Promise<EditableStrategyGroupLoadResult> => {
  const warnings: StrategyGroupLoadWarning[] = []
  let originGroup: IProxyGroupConfig | undefined
  let sequence: GroupSequence = EMPTY_GROUP_SEQUENCE

  if (profileUid) {
    try {
      const profileData = await readProfileFile(profileUid)
      const profileObject = yaml.load(profileData) as
        | { 'proxy-groups'?: IProxyGroupConfig[] }
        | null
      const originGroups = profileObject?.['proxy-groups'] || []
      originGroup = originGroups.find((item) => item?.name === group.name)
    } catch {
      warnings.push('profileReadFailed')
    }
  }

  if (property) {
    try {
      const groupsData = await readProfileFile(property)
      sequence = parseGroupsYaml(groupsData)
    } catch {
      warnings.push('groupsReadFailed')
    }
  } else {
    warnings.push('profileNotReady')
  }

  const overrideGroup =
    findLastGroupByName(sequence.append, group.name) ||
    findLastGroupByName(sequence.prepend, group.name)
  const baseGroup = cloneGroupConfig(
    overrideGroup || originGroup || buildFallbackGroupConfig(group),
  )
  const selectedNames = normalizeNames(
    Array.isArray(overrideGroup?.proxies) ? overrideGroup.proxies : [],
  ).filter((name) => !isHiddenProxyName(name) && !isBuiltinPolicyName(name))

  return {
    sequence,
    state: {
      baseGroup,
      originExists: Boolean(originGroup),
    },
    selectedNames,
    warnings,
  }
}

const resolveLoadWarningMessage = (
  warnings: StrategyGroupLoadWarning[],
): string => {
  if (warnings.includes('profileNotReady')) {
    return '当前策略池覆盖配置还没准备好，先展示全部节点；配置加载完成后即可保存。'
  }

  if (warnings.includes('groupsReadFailed')) {
    return '策略池已有配置暂时没读到，先展示全部节点；保存后会按当前勾选重建。'
  }

  if (warnings.includes('profileReadFailed')) {
    return '原始分组配置暂时没读到，先展示全部节点；保存时会按当前勾选写入策略池。'
  }

  return ''
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
  const [loadWarning, setLoadWarning] = useState('')

  const groupsProperty = current?.option?.groups?.trim() || ''
  const profileUid = current?.uid?.trim() || ''

  useEffect(() => {
    if (!open || !group) {
      setLoading(false)
      setSaving(false)
      setSearchText('')
      setSelectedNames([])
      setLoadWarning('')
      return
    }

    let cancelled = false

    setLoading(true)
    setSaving(false)
    setSearchText('')
    setSelectedNames([])
    setLoadWarning(
      resolveLoadWarningMessage(groupsProperty ? [] : ['profileNotReady']),
    )

    void (async () => {
      let result = await loadEditableStrategyGroup(
        group,
        profileUid,
        groupsProperty,
      )

      if (
        groupsProperty &&
        result.warnings.includes('groupsReadFailed') &&
        (await enhanceProfiles())
      ) {
        result = await loadEditableStrategyGroup(
          group,
          profileUid,
          groupsProperty,
        )
      }

      if (cancelled) return

      setSelectedNames(result.selectedNames)
      setLoadWarning(resolveLoadWarningMessage(result.warnings))
    })()
      .catch(() => {
        if (cancelled) return
        setSelectedNames([])
        setLoadWarning(
          '策略池配置暂时读取失败，先展示全部节点；配置恢复后可以继续保存。',
        )
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [group, groupsProperty, open, profileUid])

  const candidateOptions = useMemo(() => {
    const records = (proxiesData?.records || {}) as Record<string, IProxyItem>
    const selectedOrder = new Map(
      selectedNames.map((name, index) => [name, index]),
    )
    const options = new Map<string, CandidateOption>()

    ;(Object.values(records) as IProxyItem[]).forEach((record) => {
      if (!record?.name) return
      if (isBuiltinPolicyName(record.name)) return
      if (isHiddenProxyName(record.name)) return
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
      if (isHiddenProxyName(name) || isBuiltinPolicyName(name)) return
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

    if (!groupsProperty) {
      showNotice.error(
        '当前策略池覆盖配置还没准备好，暂时无法保存，请稍后再试。',
      )
      return
    }

    if (selectedNames.length === 0) {
      showNotice.error('策略池至少要保留一个成员。')
      return
    }

    setSaving(true)

    try {
      const result = await loadEditableStrategyGroup(
        group,
        profileUid,
        groupsProperty,
      )
      const nextGroup = cloneGroupConfig(result.state.baseGroup)

      nextGroup.proxies = [...selectedNames]
      delete nextGroup.use
      delete nextGroup['include-all']
      delete nextGroup['include-all-proxies']
      delete nextGroup['include-all-providers']
      delete nextGroup.filter
      delete nextGroup['exclude-filter']
      delete nextGroup['exclude-type']

      const nextPrepend = result.sequence.prepend.filter(
        (item) => item?.name !== group.name,
      )
      const nextAppend = result.sequence.append.filter(
        (item) => item?.name !== group.name,
      )
      const nextDelete = result.sequence.delete.filter(
        (name) => name !== group.name,
      )

      if (result.state.originExists) {
        nextDelete.push(group.name)
      }

      const nextYaml = buildGroupsYaml(
        nextPrepend,
        [...nextAppend, nextGroup],
        normalizeNames(nextDelete),
      )

      if (!(await saveProfileFile(groupsProperty, nextYaml))) {
        throw new Error('策略池成员保存失败。')
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
  const canSave =
    !loading &&
    !saving &&
    selectedNames.length > 0 &&
    Boolean(groupsProperty)

  return (
    <Dialog
      open={open}
      onClose={onClose}
      showCloseButton
      maxWidth="md"
      fullWidth
      slotProps={{ paper: { className: 'max-h-full' } }}
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

        {loadWarning ? (
          <div className="rounded-xl border border-amber-500/20 bg-amber-500/10 px-3 py-2 text-xs text-amber-200">
            {loadWarning}
          </div>
        ) : null}

        <div className="rounded-2xl border border-white/8 bg-white/5 p-2">
          {loading ? (
            <div className="px-4 pb-2 pt-1 text-xs text-text-secondary">
              正在读取当前策略池已保存成员...
            </div>
          ) : null}

          {candidateOptions.length === 0 ? (
            <div className="px-4 py-10 text-center text-sm text-text-secondary">
              没有匹配到可加入的节点。
            </div>
          ) : (
            <div className="max-h-[50vh] overflow-y-auto pr-1">
              <div className="grid grid-cols-3 items-start gap-2.5">
                {candidateOptions.map((option) => {
                  const checked = selectedNameSet.has(option.name)

                  return (
                    <ListItemButton
                      key={option.name}
                      selected={checked}
                      className={cn(
                        'self-start items-start rounded-xl border border-white/8 px-3 py-2',
                        checked
                          ? 'bg-primary/10'
                          : 'bg-black/10 hover:bg-white/8',
                      )}
                      onClick={() => toggleSelected(option.name)}
                    >
                      <div
                        className="mr-2 mt-0.5"
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
                        slotProps={{
                          secondary: {
                            className:
                              'mt-0.5 text-[11px] uppercase tracking-wide text-text-secondary',
                          },
                        }}
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
