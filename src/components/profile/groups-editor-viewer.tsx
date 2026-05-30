import {
  DndContext,
  DragEndEvent,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
} from '@dnd-kit/core'
import { SortableContext, sortableKeyboardCoordinates } from '@dnd-kit/sortable'
import { useLockFn } from 'ahooks'
import {
  cancelIdleCallback,
  requestIdleCallback,
} from 'foxact/request-idle-callback'
import yaml from 'js-yaml'
import {
  startTransition,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
} from 'react'
import { Controller, useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'

import {
  BaseSearchBox,
  MonacoEditor,
  Switch,
  VirtualList,
} from '@/components/base'
import { GroupItem } from '@/components/profile/group-item'
import { Button } from '@/components/tailwind/Button'
import { Dialog } from '@/components/tailwind/Dialog'
import { DialogActions } from '@/components/tailwind/DialogActions'
import { DialogContent } from '@/components/tailwind/DialogContent'
import { DialogTitle } from '@/components/tailwind/DialogTitle'
import { InputAdornment } from '@/components/tailwind/InputAdornment'
import { List } from '@/components/tailwind/List'
import { ListItem } from '@/components/tailwind/ListItem'
import { ListItemText } from '@/components/tailwind/ListItemText'
import { Select } from '@/components/tailwind/Select'
import type { SelectChangeEvent } from '@/components/tailwind/Select'
import { TextField } from '@/components/tailwind/TextField'
import {
  getNetworkInterfaces,
  readProfileFile,
  saveProfileFile,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import type { TranslationKey } from '@/types/generated/i18n-keys'
import type { MonacoEditorInstance } from '@/types/monaco'

interface Props {
  proxiesUid: string
  mergeUid: string
  profileUid: string
  property: string
  open: boolean
  onClose: () => void
  onSave?: (prev?: string, curr?: string) => void
}

const builtinProxyPolicies = ['DIRECT', 'REJECT', 'REJECT-DROP', 'PASS']

const PROXY_STRATEGY_LABEL_KEYS: Record<string, TranslationKey> = {
  select: 'proxies.components.enums.strategies.select',
  'url-test': 'proxies.components.enums.strategies.url-test',
  fallback: 'proxies.components.enums.strategies.fallback',
  'load-balance': 'proxies.components.enums.strategies.load-balance',
  relay: 'proxies.components.enums.strategies.relay',
}

const PROXY_POLICY_LABEL_KEYS: Record<string, TranslationKey> =
  builtinProxyPolicies.reduce(
    (acc, policy) => {
      acc[policy] =
        `proxies.components.enums.policies.${policy}` as TranslationKey
      return acc
    },
    {} as Record<string, TranslationKey>,
  )

const normalizeDeleteSeq = (input?: unknown): string[] => {
  if (!Array.isArray(input)) {
    return []
  }

  const names = input
    .map((item) => {
      if (typeof item === 'string') {
        return item
      }

      if (
        item &&
        typeof item === 'object' &&
        'name' in item &&
        typeof (item as { name: unknown }).name === 'string'
      ) {
        return (item as { name: string }).name
      }

      return undefined
    })
    .filter(
      (name): name is string => typeof name === 'string' && name.length > 0,
    )

  return Array.from(new Set(names))
}

const buildGroupsYaml = (
  prepend: IProxyGroupConfig[],
  append: IProxyGroupConfig[],
  deleteList: string[],
) => {
  return yaml.dump(
    {
      prepend,
      append,
      delete: deleteList,
    },
    { forceQuotes: true },
  )
}

export const GroupsEditorViewer = (props: Props) => {
  const { mergeUid, proxiesUid, profileUid, property, open, onClose, onSave } =
    props
  const { t } = useTranslation()
  const translateStrategy = useCallback(
    (value: string) =>
      PROXY_STRATEGY_LABEL_KEYS[value]
        ? t(PROXY_STRATEGY_LABEL_KEYS[value])
        : value,
    [t],
  )
  const translatePolicy = useCallback(
    (value: string) =>
      PROXY_POLICY_LABEL_KEYS[value]
        ? t(PROXY_POLICY_LABEL_KEYS[value])
        : value,
    [t],
  )
  const themeMode: 'dark' = 'dark'
  const editorRef = useRef<MonacoEditorInstance | null>(null)
  const [prevData, setPrevData] = useState('')
  const [currData, setCurrData] = useState('')
  const [visualization, setVisualization] = useState(true)
  const [match, setMatch] = useState(() => (_: string) => true)
  const [interfaceNameList, setInterfaceNameList] = useState<string[]>([])
  const { control, ...formIns } = useForm<IProxyGroupConfig>({
    defaultValues: {
      type: 'select',
      name: '',
      interval: 300,
      timeout: 5000,
      'max-failed-times': 5,
      lazy: true,
    },
  })
  const [groupList, setGroupList] = useState<IProxyGroupConfig[]>([])
  const [proxyPolicyList, setProxyPolicyList] = useState<string[]>([])
  const [proxyProviderList, setProxyProviderList] = useState<string[]>([])
  const [prependSeq, setPrependSeq] = useState<IProxyGroupConfig[]>([])
  const [appendSeq, setAppendSeq] = useState<IProxyGroupConfig[]>([])
  const [deleteSeq, setDeleteSeq] = useState<string[]>([])

  const filteredPrependSeq = useMemo(
    () => prependSeq.filter((group) => match(group.name)),
    [prependSeq, match],
  )
  const filteredGroupList = useMemo(
    () => groupList.filter((group) => match(group.name)),
    [groupList, match],
  )
  const filteredAppendSeq = useMemo(
    () => appendSeq.filter((group) => match(group.name)),
    [appendSeq, match],
  )

  const renderItem = (index: number): React.ReactNode => {
    const shift = filteredPrependSeq.length > 0 ? 1 : 0
    if (filteredPrependSeq.length > 0 && index === 0) {
      return (
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragEnd={onPrependDragEnd}
        >
          <SortableContext
            items={filteredPrependSeq.map((x) => {
              return x.name
            })}
          >
            {filteredPrependSeq.map((item) => {
              return (
                <GroupItem
                  key={item.name}
                  type="prepend"
                  group={item}
                  onDelete={() => {
                    setPrependSeq(
                      prependSeq.filter((v) => v.name !== item.name),
                    )
                  }}
                />
              )
            })}
          </SortableContext>
        </DndContext>
      )
    } else if (index < filteredGroupList.length + shift) {
      const newIndex = index - shift
      return (
        <GroupItem
          key={filteredGroupList[newIndex].name}
          type={
            deleteSeq.includes(filteredGroupList[newIndex].name)
              ? 'delete'
              : 'original'
          }
          group={filteredGroupList[newIndex]}
          onDelete={() => {
            if (deleteSeq.includes(filteredGroupList[newIndex].name)) {
              setDeleteSeq(
                deleteSeq.filter((v) => v !== filteredGroupList[newIndex].name),
              )
            } else {
              setDeleteSeq((prev) => [
                ...prev,
                filteredGroupList[newIndex].name,
              ])
            }
          }}
        />
      )
    } else {
      return (
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragEnd={onAppendDragEnd}
        >
          <SortableContext
            items={filteredAppendSeq.map((x) => {
              return x.name
            })}
          >
            {filteredAppendSeq.map((item) => {
              return (
                <GroupItem
                  key={item.name}
                  type="append"
                  group={item}
                  onDelete={() => {
                    setAppendSeq(appendSeq.filter((v) => v.name !== item.name))
                  }}
                />
              )
            })}
          </SortableContext>
        </DndContext>
      )
    }
  }

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  )
  const reorder = (
    list: IProxyGroupConfig[],
    startIndex: number,
    endIndex: number,
  ) => {
    const result = Array.from(list)
    const [removed] = result.splice(startIndex, 1)
    result.splice(endIndex, 0, removed)
    return result
  }
  const onPrependDragEnd = async (event: DragEndEvent) => {
    const { active, over } = event
    if (over) {
      if (active.id !== over.id) {
        let activeIndex = 0
        let overIndex = 0
        prependSeq.forEach((item, index) => {
          if (item.name === active.id) {
            activeIndex = index
          }
          if (item.name === over.id) {
            overIndex = index
          }
        })

        setPrependSeq(reorder(prependSeq, activeIndex, overIndex))
      }
    }
  }
  const onAppendDragEnd = async (event: DragEndEvent) => {
    const { active, over } = event
    if (over) {
      if (active.id !== over.id) {
        let activeIndex = 0
        let overIndex = 0
        appendSeq.forEach((item, index) => {
          if (item.name === active.id) {
            activeIndex = index
          }
          if (item.name === over.id) {
            overIndex = index
          }
        })
        setAppendSeq(reorder(appendSeq, activeIndex, overIndex))
      }
    }
  }
  const fetchContent = useCallback(async () => {
    const data = await readProfileFile(property)
    const obj = yaml.load(data) as ISeqProfileConfig | null

    setPrependSeq(obj?.prepend || [])
    setAppendSeq(obj?.append || [])
    setDeleteSeq((prev) => {
      const normalized = normalizeDeleteSeq(obj?.delete)
      if (
        normalized.length === prev.length &&
        normalized.every((item, index) => item === prev[index])
      ) {
        return prev
      }
      return normalized
    })

    setPrevData(data)
    setCurrData(data)
  }, [property])

  useEffect(() => {
    if (currData === '' || visualization !== true) {
      return
    }

    const obj = yaml.load(currData) as ISeqProfileConfig | null
    startTransition(() => {
      setPrependSeq(obj?.prepend ?? [])
      setAppendSeq(obj?.append ?? [])
      setDeleteSeq((prev) => {
        const normalized = normalizeDeleteSeq(obj?.delete)
        if (
          normalized.length === prev.length &&
          normalized.every((item, index) => item === prev[index])
        ) {
          return prev
        }
        return normalized
      })
    })
  }, [currData, visualization])

  // 优化：异步处理大数据yaml.dump，避免UI卡死
  useEffect(() => {
    if (prependSeq && appendSeq && deleteSeq) {
      const serialize = () => {
        try {
          setCurrData(buildGroupsYaml(prependSeq, appendSeq, deleteSeq))
        } catch (e) {
          console.warn('[GroupsEditorViewer] yaml.dump failed:', e)
          // 防止异常导致UI卡死
        }
      }

      const handle = requestIdleCallback(serialize)
      return () => {
        cancelIdleCallback(handle)
      }
    }
  }, [prependSeq, appendSeq, deleteSeq])

  const fetchProxyPolicy = useCallback(async () => {
    const data = await readProfileFile(profileUid)
    const proxiesData = await readProfileFile(proxiesUid)
    const originGroupsObj = yaml.load(data) as {
      'proxy-groups': IProxyGroupConfig[]
    } | null

    const originProxiesObj = yaml.load(data) as { proxies: [] } | null
    const originProxies = originProxiesObj?.proxies || []
    const moreProxiesObj = yaml.load(proxiesData) as ISeqProfileConfig | null
    const morePrependProxies = moreProxiesObj?.prepend || []
    const moreAppendProxies = moreProxiesObj?.append || []
    const moreDeleteProxies = normalizeDeleteSeq(moreProxiesObj?.delete)

    const proxies = morePrependProxies.concat(
      originProxies.filter((proxy: any) => {
        const proxyName =
          typeof proxy === 'string'
            ? proxy
            : (proxy?.name as string | undefined)
        return proxyName ? !moreDeleteProxies.includes(proxyName) : true
      }),
      moreAppendProxies,
    )

    const proxyNames = proxies
      .map((proxy: any) =>
        typeof proxy === 'string' ? proxy : (proxy?.name as string | undefined),
      )
      .filter(
        (name): name is string => typeof name === 'string' && name.length > 0,
      )

    const computedPolicyList = builtinProxyPolicies.concat(
      prependSeq.map((group: IProxyGroupConfig) => group.name),
      (originGroupsObj?.['proxy-groups'] || [])
        .map((group: IProxyGroupConfig) => group.name)
        .filter((name) => !deleteSeq.includes(name)),
      appendSeq.map((group: IProxyGroupConfig) => group.name),
      proxyNames,
    )

    setProxyPolicyList(Array.from(new Set(computedPolicyList)))
  }, [appendSeq, deleteSeq, prependSeq, profileUid, proxiesUid])
  const fetchProfile = useCallback(async () => {
    const data = await readProfileFile(profileUid)
    const mergeData = await readProfileFile(mergeUid)
    const globalMergeData = await readProfileFile('Merge')

    const originGroupsObj = yaml.load(data) as {
      'proxy-groups': IProxyGroupConfig[]
    } | null

    const originProviderObj = yaml.load(data) as {
      'proxy-providers': Record<string, unknown>
    } | null
    const originProvider = originProviderObj?.['proxy-providers'] || {}

    const moreProviderObj = yaml.load(mergeData) as {
      'proxy-providers': Record<string, unknown>
    } | null
    const moreProvider = moreProviderObj?.['proxy-providers'] || {}

    const globalProviderObj = yaml.load(globalMergeData) as {
      'proxy-providers': Record<string, unknown>
    } | null
    const globalProvider = globalProviderObj?.['proxy-providers'] || {}

    const provider = Object.assign(
      {},
      originProvider,
      moreProvider,
      globalProvider,
    )

    setProxyProviderList(Object.keys(provider))
    setGroupList(originGroupsObj?.['proxy-groups'] || [])
  }, [mergeUid, profileUid])
  const getInterfaceNameList = useCallback(async () => {
    const list = await getNetworkInterfaces()
    setInterfaceNameList(list)
  }, [])
  useEffect(() => {
    if (!open) return
    fetchProxyPolicy()
  }, [fetchProxyPolicy, open])

  useEffect(() => {
    if (!open) return
    fetchContent()
    fetchProfile()
    getInterfaceNameList()
  }, [fetchContent, fetchProfile, getInterfaceNameList, open])

  useEffect(() => {
    return () => {
      editorRef.current?.dispose()
      editorRef.current = null
    }
  }, [])

  const validateGroup = () => {
    const group = formIns.getValues()
    if (group.name === '') {
      throw new Error(t('profiles.modals.groupsEditor.errors.nameRequired'))
    }
  }

  const handleSave = useLockFn(async () => {
    try {
      const nextData = visualization
        ? buildGroupsYaml(prependSeq, appendSeq, deleteSeq)
        : currData

      if (visualization) {
        setCurrData(nextData)
      }

      if (!(await saveProfileFile(property, nextData))) {
        await fetchContent()
        onClose()
        return
      }
      showNotice.success('shared.feedback.notifications.saved')
      setPrevData(nextData)
      onSave?.(prevData, nextData)
      onClose()
    } catch (err) {
      showNotice.error(err)
    }
  })

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth="xl"
      fullWidth
      disableEnforceFocus={!visualization}
    >
      <DialogTitle>
        <div className="flex justify-between">
          <span>{t('profiles.modals.groupsEditor.title')}</span>
          <Button
            variant="contained"
            size="small"
            onClick={() => {
              setVisualization((prev) => !prev)
            }}
          >
            {visualization
              ? t('shared.editorModes.advanced')
              : t('shared.editorModes.visualization')}
          </Button>
        </div>
      </DialogTitle>

      <DialogContent className="flex w-auto h-[calc(100vh-185px)]">
        {visualization ? (
          <>
            <List className="w-1/2 px-2.5">
              <div className="h-[calc(100%-80px)] overflow-y-auto">
                <Controller
                  name="type"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t('profiles.modals.groupsEditor.fields.type')}
                      />
                      <Select
                        size="small"
                        className="w-[calc(100%-150px)]"
                        value={field.value}
                        onChange={(e) => field.onChange(e.target.value)}
                      >
                        {[
                          'select',
                          'url-test',
                          'fallback',
                          'load-balance',
                          'relay',
                        ].map((option) => (
                          <option key={option} value={option}>
                            {translateStrategy(option)}
                          </option>
                        ))}
                      </Select>
                    </ListItem>
                  )}
                />
                <Controller
                  name="name"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t('profiles.modals.groupsEditor.fields.name')}
                      />
                      <TextField
                        autoComplete="new-password"
                        size="small"
                        className="w-[calc(100%-150px)]"
                        {...field}
                        error={field.value === ''}
                        required={true}
                      />
                    </ListItem>
                  )}
                />
                <Controller
                  name="icon"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t('profiles.modals.groupsEditor.fields.icon')}
                      />
                      <TextField
                        autoComplete="new-password"
                        size="small"
                        className="w-[calc(100%-150px)]"
                        {...field}
                      />
                    </ListItem>
                  )}
                />
                <Controller
                  name="proxies"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.fields.proxies',
                        )}
                      />
                      <Select
                        size="small"
                        className="w-[calc(100%-150px)]"
                        multiple
                        value={field.value || []}
                        onChange={(e) => {
                          const value = e.target.value
                          field.onChange(
                            typeof value === 'string' ? value.split(',') : value,
                          )
                        }}
                      >
                        {proxyPolicyList.map((option) => (
                          <option key={option} value={option}>
                            {translatePolicy(option)}
                          </option>
                        ))}
                      </Select>
                    </ListItem>
                  )}
                />
                <Controller
                  name="use"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.fields.provider',
                        )}
                      />
                      <Select
                        size="small"
                        className="w-[calc(100%-150px)]"
                        multiple
                        value={field.value || []}
                        onChange={(e) => {
                          const value = e.target.value
                          field.onChange(
                            typeof value === 'string' ? value.split(',') : value,
                          )
                        }}
                      >
                        {proxyProviderList.map((option) => (
                          <option key={option} value={option}>
                            {option}
                          </option>
                        ))}
                      </Select>
                    </ListItem>
                  )}
                />
                <Controller
                  name="url"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.fields.healthCheckUrl',
                        )}
                      />
                      <TextField
                        autoComplete="new-password"
                        placeholder="http://cp.cloudflare.com/generate_204"
                        size="small"
                        className="w-[calc(100%-150px)]"
                        {...field}
                      />
                    </ListItem>
                  )}
                />
                <Controller
                  name="expected-status"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.fields.expectedStatus',
                        )}
                      />
                      <TextField
                        autoComplete="new-password"
                        placeholder="*"
                        size="small"
                        className="w-[calc(100%-150px)]"
                        onChange={(e: ChangeEvent<HTMLInputElement>) => {
                          field.onChange(parseInt(e.target.value))
                        }}
                      />
                    </ListItem>
                  )}
                />
                <Controller
                  name="interval"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.fields.interval',
                        )}
                      />
                      <TextField
                        autoComplete="new-password"
                        placeholder="300"
                        type="number"
                        size="small"
                        className="w-[calc(100%-150px)]"
                        onChange={(e: ChangeEvent<HTMLInputElement>) => {
                          field.onChange(parseInt(e.target.value))
                        }}
                        InputProps={{
                          endAdornment: (
                            <InputAdornment position="end">
                              {t('shared.units.seconds')}
                            </InputAdornment>
                          ),
                        }}
                      />
                    </ListItem>
                  )}
                />
                <Controller
                  name="timeout"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText primary={t('shared.labels.timeout')} />
                      <TextField
                        autoComplete="new-password"
                        placeholder="5000"
                        type="number"
                        size="small"
                        className="w-[calc(100%-150px)]"
                        onChange={(e: ChangeEvent<HTMLInputElement>) => {
                          field.onChange(parseInt(e.target.value))
                        }}
                        InputProps={{
                          endAdornment: (
                            <InputAdornment position="end">
                              {t('shared.units.milliseconds')}
                            </InputAdornment>
                          ),
                        }}
                      />
                    </ListItem>
                  )}
                />
                <Controller
                  name="max-failed-times"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.fields.maxFailedTimes',
                        )}
                      />
                      <TextField
                        autoComplete="new-password"
                        placeholder="5"
                        type="number"
                        size="small"
                        className="w-[calc(100%-150px)]"
                        onChange={(e: ChangeEvent<HTMLInputElement>) => {
                          field.onChange(parseInt(e.target.value))
                        }}
                      />
                    </ListItem>
                  )}
                />
                <Controller
                  name="interface-name"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.fields.interfaceName',
                        )}
                      />
                      <Select
                        size="small"
                        className="w-[calc(100%-150px)]"
                        value={field.value || ''}
                        onChange={(e: SelectChangeEvent) => field.onChange(e.target.value)}
                      >
                        {interfaceNameList.map((option) => (
                          <option key={option} value={option}>
                            {option}
                          </option>
                        ))}
                      </Select>
                    </ListItem>
                  )}
                />
                <Controller
                  name="routing-mark"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.fields.routingMark',
                        )}
                      />
                      <TextField
                        autoComplete="new-password"
                        type="number"
                        size="small"
                        className="w-[calc(100%-150px)]"
                        onChange={(e: ChangeEvent<HTMLInputElement>) => {
                          field.onChange(parseInt(e.target.value))
                        }}
                      />
                    </ListItem>
                  )}
                />
                <Controller
                  name="filter"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.fields.filter',
                        )}
                      />
                      <TextField
                        autoComplete="new-password"
                        size="small"
                        className="w-[calc(100%-150px)]"
                        {...field}
                      />
                    </ListItem>
                  )}
                />
                <Controller
                  name="exclude-filter"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.fields.excludeFilter',
                        )}
                      />
                      <TextField
                        autoComplete="new-password"
                        size="small"
                        className="w-[calc(100%-150px)]"
                        {...field}
                      />
                    </ListItem>
                  )}
                />
                <Controller
                  name="exclude-type"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.fields.excludeType',
                        )}
                      />
                      <Select
                        multiple
                        size="small"
                        className="w-[calc(100%-150px)]"
                        value={field.value?.split('|') || []}
                        onChange={(e) => {
                          const value = e.target.value
                          const arr =
                            typeof value === 'string' ? value.split(',') : value
                          field.onChange(arr.join('|'))
                        }}
                      >
                        {[
                          'Direct',
                          'Reject',
                          'RejectDrop',
                          'Compatible',
                          'Pass',
                          'Dns',
                          'Shadowsocks',
                          'ShadowsocksR',
                          'Snell',
                          'Socks5',
                          'Http',
                          'Vmess',
                          'Vless',
                          'Trojan',
                          'Hysteria',
                          'Hysteria2',
                          'WireGuard',
                          'Tuic',
                          'Mieru',
                          'Masque',
                          'AnyTLS',
                          'Sudoku',
                          'Relay',
                          'Selector',
                          'Fallback',
                          'URLTest',
                          'LoadBalance',
                          'Ssh',
                        ].map((option) => (
                          <option key={option} value={option}>
                            {option}
                          </option>
                        ))}
                      </Select>
                    </ListItem>
                  )}
                />
                <Controller
                  name="include-all"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.fields.includeAll',
                        )}
                      />
                      <Switch checked={field.value} {...field} />
                    </ListItem>
                  )}
                />
                <Controller
                  name="include-all-proxies"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.fields.includeAllProxies',
                        )}
                      />
                      <Switch checked={field.value} {...field} />
                    </ListItem>
                  )}
                />
                <Controller
                  name="include-all-providers"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.fields.includeAllProviders',
                        )}
                      />
                      <Switch checked={field.value} {...field} />
                    </ListItem>
                  )}
                />
                <Controller
                  name="lazy"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t('profiles.modals.groupsEditor.toggles.lazy')}
                      />
                      <Switch checked={field.value} {...field} />
                    </ListItem>
                  )}
                />
                <Controller
                  name="disable-udp"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.toggles.disableUdp',
                        )}
                      />
                      <Switch checked={field.value} {...field} />
                    </ListItem>
                  )}
                />
                <Controller
                  name="hidden"
                  control={control}
                  render={({ field }) => (
                    <ListItem className="py-1.5 px-0.5">
                      <ListItemText
                        primary={t(
                          'profiles.modals.groupsEditor.toggles.hidden',
                        )}
                      />
                      <Switch checked={field.value} {...field} />
                    </ListItem>
                  )}
                />
              </div>
              <ListItem className="py-1.5 px-0.5">
                <Button
                  fullWidth
                  variant="contained"
                  onClick={() => {
                    try {
                      validateGroup()
                      for (const item of [...prependSeq, ...groupList]) {
                        if (item.name === formIns.getValues().name) {
                          throw new Error(
                            t('profiles.modals.groupsEditor.errors.nameExists'),
                          )
                        }
                      }
                      setPrependSeq([formIns.getValues(), ...prependSeq])
                    } catch (err) {
                      showNotice.error(err)
                    }
                  }}
                >
                  <svg
                    className="w-5 h-5 mr-2"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path d="M8 11h3v10h2V11h3l-4-4-4 4zM4 3v2h16V3H4z" />
                  </svg>
                  {t('profiles.modals.groupsEditor.actions.prepend')}
                </Button>
              </ListItem>
              <ListItem className="py-1.5 px-0.5">
                <Button
                  fullWidth
                  variant="contained"
                  onClick={() => {
                    try {
                      validateGroup()
                      for (const item of [...appendSeq, ...groupList]) {
                        if (item.name === formIns.getValues().name) {
                          throw new Error(
                            t('profiles.modals.groupsEditor.errors.nameExists'),
                          )
                        }
                      }
                      setAppendSeq([...appendSeq, formIns.getValues()])
                    } catch (err) {
                      showNotice.error(err)
                    }
                  }}
                >
                  <svg
                    className="w-5 h-5 mr-2"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path d="M16 13h-3V3h-2v10H8l4 4 4-4zM4 19v2h16v-2H4z" />
                  </svg>
                  {t('profiles.modals.groupsEditor.actions.append')}
                </Button>
              </ListItem>
            </List>

            <List className="w-1/2 px-2.5">
              <BaseSearchBox onSearch={(match) => setMatch(() => match)} />
              <VirtualList
                count={
                  filteredGroupList.length +
                  (filteredPrependSeq.length > 0 ? 1 : 0) +
                  (filteredAppendSeq.length > 0 ? 1 : 0)
                }
                estimateSize={56}
                renderItem={renderItem}
                style={{ height: 'calc(100% - 24px)', marginTop: '8px' }}
              />
            </List>
          </>
        ) : (
          <MonacoEditor
            height="100%"
            language="yaml"
            value={currData}
            theme='vs-dark'
            onMount={(editorInstance) => {
              editorRef.current = editorInstance
            }}
            options={{
              tabSize: 2,
              minimap: {
                enabled: document.documentElement.clientWidth >= 1500,
              },
              mouseWheelZoom: true,
              quickSuggestions: {
                strings: true,
                comments: true,
                other: true,
              },
              padding: {
                top: 33,
              },
              fontFamily:
                'Josefin Sans, YouSheBiaoTiHei, twemoji mozilla, Segoe UI Emoji, -apple-system, BlinkMacSystemFont, Segoe UI, Microsoft YaHei UI, Microsoft YaHei, Roboto, Helvetica Neue, Arial, sans-serif',
              fontLigatures: false,
              smoothScrolling: true,
            }}
            onChange={(value) => setCurrData(value ?? '')}
          />
        )}
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose} variant="outlined">
          {t('shared.actions.cancel')}
        </Button>

        <Button onClick={handleSave} variant="contained">
          {t('shared.actions.save')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
