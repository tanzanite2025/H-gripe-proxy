import {
  cancelIdleCallback,
  requestIdleCallback,
} from 'foxact/request-idle-callback'
import yaml from 'js-yaml'
import { startTransition, useCallback, useEffect, useState } from 'react'

import {
  getNetworkInterfaces,
  readProfileFile,
} from '@/services/cmds'

import {
  buildGroupsYaml,
  normalizeDeleteSeq,
} from '../utils/group-helpers'
import { builtinProxyPolicies } from '../constants'

interface UseGroupDataProps {
  mergeUid: string
  proxiesUid: string
  profileUid: string
  property: string
  open: boolean
  visualization: boolean
  currData: string
  setCurrData: (data: string) => void
}

export const useGroupData = ({
  mergeUid,
  proxiesUid,
  profileUid,
  property,
  open,
  visualization,
  currData,
  setCurrData,
}: UseGroupDataProps) => {
  const [prevData, setPrevData] = useState('')
  const [groupList, setGroupList] = useState<IProxyGroupConfig[]>([])
  const [proxyPolicyList, setProxyPolicyList] = useState<string[]>([])
  const [proxyProviderList, setProxyProviderList] = useState<string[]>([])
  const [prependSeq, setPrependSeq] = useState<IProxyGroupConfig[]>([])
  const [appendSeq, setAppendSeq] = useState<IProxyGroupConfig[]>([])
  const [deleteSeq, setDeleteSeq] = useState<string[]>([])
  const [interfaceNameList, setInterfaceNameList] = useState<string[]>([])

  // Fetch content from profile file
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
  }, [property, setCurrData])

  // Parse currData when switching to visualization mode
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

  // Serialize sequences to YAML (async with idle callback)
  useEffect(() => {
    if (prependSeq && appendSeq && deleteSeq) {
      const serialize = () => {
        try {
          setCurrData(buildGroupsYaml(prependSeq, appendSeq, deleteSeq))
        } catch (e) {
          console.warn('[GroupsEditorViewer] yaml.dump failed:', e)
        }
      }

      const handle = requestIdleCallback(serialize)
      return () => {
        cancelIdleCallback(handle)
      }
    }
  }, [prependSeq, appendSeq, deleteSeq, setCurrData])

  // Fetch proxy policy list
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

  // Fetch profile groups and providers
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

  // Fetch network interface names
  const getInterfaceNameList = useCallback(async () => {
    const list = await getNetworkInterfaces()
    setInterfaceNameList(list)
  }, [])

  // Initialize data when dialog opens
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

  return {
    prevData,
    setPrevData,
    groupList,
    proxyPolicyList,
    proxyProviderList,
    prependSeq,
    setPrependSeq,
    appendSeq,
    setAppendSeq,
    deleteSeq,
    setDeleteSeq,
    interfaceNameList,
    fetchContent,
  }
}
