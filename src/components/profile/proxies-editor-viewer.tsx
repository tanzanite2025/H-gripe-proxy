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
import yaml from 'js-yaml'
import {
  startTransition,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { useTranslation } from 'react-i18next'

import { BaseSearchBox, MonacoEditor, VirtualList } from '@/components/base'
import { ProxyItem } from '@/components/profile/proxy-item'
import { Button } from '@/components/tailwind/Button'
import {
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
} from '@/components/tailwind/Dialog'
import { List, ListItem } from '@/components/tailwind/List'
import { TextField } from '@/components/tailwind/TextField'
import { readProfileFile, saveProfileFile } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { useThemeMode } from '@/services/states'
import type { MonacoEditorInstance } from '@/types/monaco'
import getSystem from '@/utils/misc'
import parseUri from '@/utils/parser/uri'

interface Props {
  profileUid: string
  property: string
  open: boolean
  onClose: () => void
  onSave?: (prev?: string, curr?: string) => void
}

export const ProxiesEditorViewer = (props: Props) => {
  const { profileUid, property, open, onClose, onSave } = props
  const { t } = useTranslation()
  const themeMode = useThemeMode()
  const editorRef = useRef<MonacoEditorInstance | null>(null)
  const [prevData, setPrevData] = useState('')
  const [currData, setCurrData] = useState('')
  const [visualization, setVisualization] = useState(true)
  const [match, setMatch] = useState(() => (_: string) => true)
  const [proxyUri, setProxyUri] = useState<string>('')

  const [proxyList, setProxyList] = useState<IProxyConfig[]>([])
  const [prependSeq, setPrependSeq] = useState<IProxyConfig[]>([])
  const [appendSeq, setAppendSeq] = useState<IProxyConfig[]>([])
  const [deleteSeq, setDeleteSeq] = useState<string[]>([])

  const filteredPrependSeq = useMemo(
    () => prependSeq.filter((proxy) => match(proxy.name)),
    [prependSeq, match],
  )
  const filteredProxyList = useMemo(
    () => proxyList.filter((proxy) => match(proxy.name)),
    [proxyList, match],
  )
  const filteredAppendSeq = useMemo(
    () => appendSeq.filter((proxy) => match(proxy.name)),
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
                <ProxyItem
                  key={item.name}
                  type="prepend"
                  proxy={item}
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
    } else if (index < filteredProxyList.length + shift) {
      const newIndex = index - shift
      return (
        <ProxyItem
          key={filteredProxyList[newIndex].name}
          type={
            deleteSeq.includes(filteredProxyList[newIndex].name)
              ? 'delete'
              : 'original'
          }
          proxy={filteredProxyList[newIndex]}
          onDelete={() => {
            if (deleteSeq.includes(filteredProxyList[newIndex].name)) {
              setDeleteSeq(
                deleteSeq.filter((v) => v !== filteredProxyList[newIndex].name),
              )
            } else {
              setDeleteSeq((prev) => [
                ...prev,
                filteredProxyList[newIndex].name,
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
                <ProxyItem
                  key={item.name}
                  type="append"
                  proxy={item}
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
    list: IProxyConfig[],
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
  // 优化：异步分片解析，避免主线程阻塞，解析完成后批量setState
  const handleParseAsync = (cb: (proxies: IProxyConfig[]) => void) => {
    const proxies: IProxyConfig[] = []
    const names: string[] = []
    let uris: string
    try {
      uris = atob(proxyUri)
    } catch {
      uris = proxyUri
    }
    const lines = uris.trim().split('\n')
    let idx = 0
    const batchSize = 50
    let parseTimer: number | undefined

    const parseBatch = () => {
      const end = Math.min(idx + batchSize, lines.length)
      for (; idx < end; idx++) {
        const uri = lines[idx]
        try {
          const proxy = parseUri(uri.trim())
          if (!names.includes(proxy.name)) {
            proxies.push(proxy)
            names.push(proxy.name)
          }
        } catch (err) {
          console.warn(
            '[ProxiesEditorViewer] parseUri failed for line:',
            uri,
            err,
          )
          // 不阻塞主流程
        }
      }
      if (idx < lines.length) {
        parseTimer = window.setTimeout(parseBatch, 0)
      } else {
        if (parseTimer !== undefined) {
          clearTimeout(parseTimer)
          parseTimer = undefined
        }
        cb(proxies)
      }
    }
    parseBatch()
  }
  const fetchProfile = useCallback(async () => {
    const data = await readProfileFile(profileUid)

    const originProxiesObj = yaml.load(data) as {
      proxies: IProxyConfig[]
    } | null

    setProxyList(originProxiesObj?.proxies || [])
  }, [profileUid])

  const fetchContent = useCallback(async () => {
    const data = await readProfileFile(property)
    const obj = yaml.load(data) as ISeqProfileConfig | null

    setPrependSeq(obj?.prepend || [])
    setAppendSeq(obj?.append || [])
    setDeleteSeq(obj?.delete || [])

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
      setDeleteSeq(obj?.delete ?? [])
    })
  }, [currData, visualization])

  useEffect(() => {
    if (!(prependSeq && appendSeq && deleteSeq)) {
      return
    }

    const serialize = () => {
      try {
        setCurrData(
          yaml.dump(
            { prepend: prependSeq, append: appendSeq, delete: deleteSeq },
            { forceQuotes: true },
          ),
        )
      } catch (e) {
        console.warn('[ProxiesEditorViewer] yaml.dump failed:', e)
        // 防止异常导致UI卡死
      }
    }
    let idleId: number | undefined
    let timeoutId: number | undefined
    if (window.requestIdleCallback) {
      idleId = window.requestIdleCallback(serialize)
    } else {
      timeoutId = window.setTimeout(serialize, 0)
    }
    return () => {
      if (idleId !== undefined && window.cancelIdleCallback) {
        window.cancelIdleCallback(idleId)
      }
      if (timeoutId !== undefined) {
        clearTimeout(timeoutId)
      }
    }
  }, [prependSeq, appendSeq, deleteSeq])

  useEffect(() => {
    if (!open) return
    fetchContent()
    fetchProfile()
  }, [fetchContent, fetchProfile, open])

  useEffect(() => {
    return () => {
      editorRef.current?.dispose()
      editorRef.current = null
    }
  }, [])

  const handleSave = useLockFn(async () => {
    try {
      if (!(await saveProfileFile(property, currData))) {
        await fetchContent()
        onClose()
        return
      }
      showNotice.success('shared.feedback.notifications.saved')
      onSave?.(prevData, currData)
      onClose()
    } catch (err) {
      showNotice.error(err)
    }
  })

  return (
    <Dialog open={open} onClose={onClose} maxWidth="xl" fullWidth>
      <DialogTitle>
        <div className="flex justify-between">
          <span>{t('profiles.modals.proxiesEditor.title')}</span>
          <Button
            variant="primary"
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
                <ListItem className="py-1.5 px-0.5">
                  <TextField
                    autoComplete="new-password"
                    placeholder={t(
                      'profiles.modals.proxiesEditor.placeholders.multiUri',
                    )}
                    className="w-full"
                    rows={9}
                    multiline
                    onChange={(e) => setProxyUri(e.target.value)}
                  />
                </ListItem>
              </div>
              <ListItem className="py-1.5 px-0.5">
                <Button
                  className="w-full"
                  variant="primary"
                  startIcon={
                    <svg
                      width="20"
                      height="20"
                      viewBox="0 0 24 24"
                      fill="currentColor"
                    >
                      <path d="M8 11h3v10h2V11h3l-4-4-4 4zM4 3v2h16V3H4z" />
                    </svg>
                  }
                  onClick={() => {
                    handleParseAsync((proxies) => {
                      setPrependSeq((prev) => [...proxies, ...prev])
                    })
                  }}
                >
                  {t('profiles.modals.proxiesEditor.actions.prepend')}
                </Button>
              </ListItem>
              <ListItem className="py-1.5 px-0.5">
                <Button
                  className="w-full"
                  variant="primary"
                  startIcon={
                    <svg
                      width="20"
                      height="20"
                      viewBox="0 0 24 24"
                      fill="currentColor"
                    >
                      <path d="M16 13h-3V3h-2v10H8l4 4 4-4zM4 19v2h16v-2H4z" />
                    </svg>
                  }
                  onClick={() => {
                    handleParseAsync((proxies) => {
                      setAppendSeq((prev) => [...prev, ...proxies])
                    })
                  }}
                >
                  {t('profiles.modals.proxiesEditor.actions.append')}
                </Button>
              </ListItem>
            </List>

            <List className="w-1/2 px-2.5">
              <BaseSearchBox onSearch={(match) => setMatch(() => match)} />
              <VirtualList
                count={
                  filteredProxyList.length +
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
            theme={themeMode === 'light' ? 'light' : 'vs-dark'}
            onMount={(editorInstance) => {
              editorRef.current = editorInstance
            }}
            options={{
              tabSize: 2, // 根据语言类型设置缩进大小
              minimap: {
                enabled: document.documentElement.clientWidth >= 1500, // 超过一定宽度显示minimap滚动条
              },
              mouseWheelZoom: true, // 按住Ctrl滚轮调节缩放比例
              quickSuggestions: {
                strings: true, // 字符串类型的建议
                comments: true, // 注释类型的建议
                other: true, // 其他类型的建议
              },
              padding: {
                top: 33, // 顶部padding防止遮挡snippets
              },
              fontFamily: `Fira Code, JetBrains Mono, Roboto Mono, "Source Code Pro", Consolas, Menlo, Monaco, monospace, "Courier New", "Apple Color Emoji"${
                getSystem() === 'windows' ? ', twemoji mozilla' : ''
              }`,
              fontLigatures: false, // 连字符
              smoothScrolling: true, // 平滑滚动
            }}
            onChange={(value) => setCurrData(value ?? '')}
          />
        )}
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose} variant="outlined">
          {t('shared.actions.cancel')}
        </Button>

        <Button onClick={handleSave} variant="primary">
          {t('shared.actions.save')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
