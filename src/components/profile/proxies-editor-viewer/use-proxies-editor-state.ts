import type { DragEndEvent } from '@dnd-kit/core'
import {
  startTransition,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from 'react'

import { readProfileFile } from '@/services/cmds'

import {
  buildVisualizationSections,
  parseProxyUriInput,
  readProfileProxyList,
  readSequenceProfileState,
  reorderProxyListByName,
  serializeSequenceProfileState,
  toggleDeletedProxy,
  tryReadSequenceProfileState,
} from './helpers'
import type {
  ProxiesEditorState,
  ProxiesEditorViewerProps,
  ProxySearchMatcher,
} from './types'

const MATCH_ALL: ProxySearchMatcher = () => true

export function useProxiesEditorState({
  open,
  profileUid,
  property,
}: Pick<ProxiesEditorViewerProps, 'open' | 'profileUid' | 'property'>): ProxiesEditorState {
  const [prevData, setPrevData] = useState('')
  const [currData, setCurrData] = useState('')
  const [visualization, setVisualization] = useState(true)
  const [match, setMatch] = useState<ProxySearchMatcher>(() => MATCH_ALL)
  const [proxyUri, setProxyUri] = useState('')

  const [proxyList, setProxyList] = useState<IProxyConfig[]>([])
  const [prependSeq, setPrependSeq] = useState<IProxyConfig[]>([])
  const [appendSeq, setAppendSeq] = useState<IProxyConfig[]>([])
  const [deleteSeq, setDeleteSeq] = useState<string[]>([])
  const [contentHydrated, setContentHydrated] = useState(false)

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
  const sections = useMemo(
    () =>
      buildVisualizationSections({
        filteredPrependSeq,
        filteredProxyList,
        filteredAppendSeq,
        deleteSeq,
      }),
    [deleteSeq, filteredAppendSeq, filteredPrependSeq, filteredProxyList],
  )

  const fetchProfile = useCallback(async () => {
    const data = await readProfileFile(profileUid)
    setProxyList(readProfileProxyList(data))
  }, [profileUid])

  const reloadContent = useCallback(async () => {
    setContentHydrated(false)

    const data = await readProfileFile(property)
    const nextState = readSequenceProfileState(data)

    setPrependSeq(nextState.prependSeq)
    setAppendSeq(nextState.appendSeq)
    setDeleteSeq(nextState.deleteSeq)
    setPrevData(data)
    setCurrData(data)
    setContentHydrated(true)
  }, [property])

  useEffect(() => {
    if (currData === '' || !visualization || !contentHydrated) {
      return
    }

    const nextState = tryReadSequenceProfileState(currData)
    if (!nextState) {
      return
    }

    startTransition(() => {
      setPrependSeq(nextState.prependSeq)
      setAppendSeq(nextState.appendSeq)
      setDeleteSeq(nextState.deleteSeq)
    })
  }, [contentHydrated, currData, visualization])

  useEffect(() => {
    if (!contentHydrated) {
      return
    }

    const serialize = () => {
      try {
        setCurrData(
          serializeSequenceProfileState({
            prependSeq,
            appendSeq,
            deleteSeq,
          }),
        )
      } catch (error) {
        console.warn('[ProxiesEditorViewer] Failed to serialize YAML:', error)
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
  }, [appendSeq, contentHydrated, deleteSeq, prependSeq])

  useEffect(() => {
    if (!open) {
      return
    }

    void Promise.all([reloadContent(), fetchProfile()]).catch((error) => {
      console.warn('[ProxiesEditorViewer] Failed to initialize editor:', error)
    })
  }, [fetchProfile, open, reloadContent])

  const toggleVisualization = useCallback(() => {
    setVisualization((value) => !value)
  }, [])

  const handleProxyUriChange = useCallback((value: string) => {
    setProxyUri(value)
  }, [])

  const handleSearchChange = useCallback((nextMatch: ProxySearchMatcher) => {
    setMatch(() => nextMatch)
  }, [])

  const handleYamlChange = useCallback((value: string) => {
    setCurrData(value)
  }, [])

  const handlePrependImport = useCallback(async () => {
    const proxies = await parseProxyUriInput(proxyUri)
    setPrependSeq((previousValue) => [...proxies, ...previousValue])
  }, [proxyUri])

  const handleAppendImport = useCallback(async () => {
    const proxies = await parseProxyUriInput(proxyUri)
    setAppendSeq((previousValue) => [...previousValue, ...proxies])
  }, [proxyUri])

  const handlePrependDelete = useCallback((name: string) => {
    setPrependSeq((previousValue) =>
      previousValue.filter((proxy) => proxy.name !== name),
    )
  }, [])

  const handleAppendDelete = useCallback((name: string) => {
    setAppendSeq((previousValue) =>
      previousValue.filter((proxy) => proxy.name !== name),
    )
  }, [])

  const handleOriginalDeleteToggle = useCallback((name: string) => {
    setDeleteSeq((previousValue) => toggleDeletedProxy(previousValue, name))
  }, [])

  const handlePrependDragEnd = useCallback((event: DragEndEvent) => {
    const { active, over } = event
    if (!over || active.id === over.id) {
      return
    }

    setPrependSeq((previousValue) =>
      reorderProxyListByName(
        previousValue,
        String(active.id),
        String(over.id),
      ),
    )
  }, [])

  const handleAppendDragEnd = useCallback((event: DragEndEvent) => {
    const { active, over } = event
    if (!over || active.id === over.id) {
      return
    }

    setAppendSeq((previousValue) =>
      reorderProxyListByName(
        previousValue,
        String(active.id),
        String(over.id),
      ),
    )
  }, [])

  return {
    prevData,
    currData,
    visualization,
    proxyUri,
    sections,
    toggleVisualization,
    handleProxyUriChange,
    handleSearchChange,
    handleYamlChange,
    handlePrependImport,
    handleAppendImport,
    handlePrependDelete,
    handleAppendDelete,
    handleOriginalDeleteToggle,
    handlePrependDragEnd,
    handleAppendDragEnd,
    reloadContent,
  }
}
