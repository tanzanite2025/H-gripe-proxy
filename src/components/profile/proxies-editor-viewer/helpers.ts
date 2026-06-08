import yaml from 'js-yaml'

import parseUri from '@/utils/parser/uri'

import type {
  ProxyVisualizationSection,
  SequenceProfileState,
} from './types'

const createEmptySequenceProfileState = (): SequenceProfileState => ({
  prependSeq: [],
  appendSeq: [],
  deleteSeq: [],
})

const normalizeSequenceProfileState = (
  profile: ISeqProfileConfig | null | undefined,
): SequenceProfileState => ({
  prependSeq: Array.isArray(profile?.prepend) ? profile.prepend : [],
  appendSeq: Array.isArray(profile?.append) ? profile.append : [],
  deleteSeq: Array.isArray(profile?.delete) ? profile.delete : [],
})

const decodeProxyUriInput = (proxyUri: string) => {
  try {
    return atob(proxyUri)
  } catch {
    return proxyUri
  }
}

export function readProfileProxyList(data: string): IProxyConfig[] {
  try {
    const profile = yaml.load(data) as
      | {
          proxies?: IProxyConfig[]
        }
      | null

    return Array.isArray(profile?.proxies) ? profile.proxies : []
  } catch (error) {
    console.warn('[ProxiesEditorViewer] Failed to parse proxy list:', error)
    return []
  }
}

export function readSequenceProfileState(data: string): SequenceProfileState {
  try {
    const profile = yaml.load(data) as ISeqProfileConfig | null
    return normalizeSequenceProfileState(profile)
  } catch (error) {
    console.warn(
      '[ProxiesEditorViewer] Failed to parse sequence profile:',
      error,
    )
    return createEmptySequenceProfileState()
  }
}

export function tryReadSequenceProfileState(
  data: string,
): SequenceProfileState | null {
  try {
    const profile = yaml.load(data) as ISeqProfileConfig | null
    return normalizeSequenceProfileState(profile)
  } catch (error) {
    console.warn(
      '[ProxiesEditorViewer] Failed to parse current editor YAML:',
      error,
    )
    return null
  }
}

export function serializeSequenceProfileState(
  state: SequenceProfileState,
): string {
  return yaml.dump(
    {
      prepend: state.prependSeq,
      append: state.appendSeq,
      delete: state.deleteSeq,
    },
    { forceQuotes: true },
  )
}

export function reorderProxyList<T>(
  list: T[],
  startIndex: number,
  endIndex: number,
): T[] {
  const result = [...list]
  const [removed] = result.splice(startIndex, 1)

  if (removed === undefined) {
    return list
  }

  result.splice(endIndex, 0, removed)
  return result
}

export function reorderProxyListByName(
  list: IProxyConfig[],
  activeId: string,
  overId: string,
): IProxyConfig[] {
  const activeIndex = list.findIndex((item) => item.name === activeId)
  const overIndex = list.findIndex((item) => item.name === overId)

  if (activeIndex === -1 || overIndex === -1 || activeIndex === overIndex) {
    return list
  }

  return reorderProxyList(list, activeIndex, overIndex)
}

export function toggleDeletedProxy(
  deleteSeq: string[],
  proxyName: string,
): string[] {
  return deleteSeq.includes(proxyName)
    ? deleteSeq.filter((value) => value !== proxyName)
    : [...deleteSeq, proxyName]
}

export function buildVisualizationSections(options: {
  filteredPrependSeq: IProxyConfig[]
  filteredProxyList: IProxyConfig[]
  filteredAppendSeq: IProxyConfig[]
  deleteSeq: string[]
}): ProxyVisualizationSection[] {
  const {
    filteredPrependSeq,
    filteredProxyList,
    filteredAppendSeq,
    deleteSeq,
  } = options
  const deletedNames = new Set(deleteSeq)
  const sections: ProxyVisualizationSection[] = []

  if (filteredPrependSeq.length > 0) {
    sections.push({ kind: 'prepend', items: filteredPrependSeq })
  }

  filteredProxyList.forEach((proxy) => {
    sections.push({
      kind: 'original',
      proxy,
      deleted: deletedNames.has(proxy.name),
    })
  })

  if (filteredAppendSeq.length > 0) {
    sections.push({ kind: 'append', items: filteredAppendSeq })
  }

  return sections
}

export function parseProxyUriInput(proxyUri: string): Promise<IProxyConfig[]> {
  return new Promise((resolve) => {
    const proxies: IProxyConfig[] = []
    const names = new Set<string>()
    const lines = decodeProxyUriInput(proxyUri)
      .trim()
      .split('\n')
      .map((line) => line.trim())
      .filter(Boolean)
    const batchSize = 50
    let index = 0
    let parseTimer: number | undefined

    // Batch parsing avoids blocking the UI when many URIs are pasted at once.
    const parseBatch = () => {
      const end = Math.min(index + batchSize, lines.length)

      for (; index < end; index += 1) {
        const uri = lines[index]

        try {
          const proxy = parseUri(uri)

          if (!names.has(proxy.name)) {
            proxies.push(proxy)
            names.add(proxy.name)
          }
        } catch (error) {
          console.warn(
            '[ProxiesEditorViewer] Failed to parse proxy URI:',
            uri,
            error,
          )
        }
      }

      if (index < lines.length) {
        parseTimer = window.setTimeout(parseBatch, 0)
        return
      }

      if (parseTimer !== undefined) {
        clearTimeout(parseTimer)
      }

      resolve(proxies)
    }

    parseBatch()
  })
}
