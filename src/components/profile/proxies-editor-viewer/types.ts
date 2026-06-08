import type { DragEndEvent } from '@dnd-kit/core'

import type { MonacoEditorInstance } from '@/types/monaco'

export interface ProxiesEditorViewerProps {
  profileUid: string
  property: string
  open: boolean
  onClose: () => void
  onSave?: (prev?: string, curr?: string) => void
}

export type ProxySearchMatcher = (input: string) => boolean
export type ProxySequenceKind = 'prepend' | 'append'

export interface SequenceProfileState {
  prependSeq: IProxyConfig[]
  appendSeq: IProxyConfig[]
  deleteSeq: string[]
}

export type ProxyVisualizationSection =
  | {
      kind: 'prepend'
      items: IProxyConfig[]
    }
  | {
      kind: 'append'
      items: IProxyConfig[]
    }
  | {
      kind: 'original'
      proxy: IProxyConfig
      deleted: boolean
    }

export interface ProxiesEditorState {
  prevData: string
  currData: string
  visualization: boolean
  proxyUri: string
  sections: ProxyVisualizationSection[]
  toggleVisualization: () => void
  handleProxyUriChange: (value: string) => void
  handleSearchChange: (match: ProxySearchMatcher) => void
  handleYamlChange: (value: string) => void
  handlePrependImport: () => Promise<void>
  handleAppendImport: () => Promise<void>
  handlePrependDelete: (name: string) => void
  handleAppendDelete: (name: string) => void
  handleOriginalDeleteToggle: (name: string) => void
  handlePrependDragEnd: (event: DragEndEvent) => void
  handleAppendDragEnd: (event: DragEndEvent) => void
  reloadContent: () => Promise<void>
}

export interface ProxyVisualizationPaneProps {
  proxyUri: string
  sections: ProxyVisualizationSection[]
  importPlaceholder: string
  prependLabel: string
  appendLabel: string
  onProxyUriChange: (value: string) => void
  onSearchChange: (match: ProxySearchMatcher) => void
  onPrependImport: () => void
  onAppendImport: () => void
  onPrependDelete: (name: string) => void
  onAppendDelete: (name: string) => void
  onOriginalDeleteToggle: (name: string) => void
  onPrependDragEnd: (event: DragEndEvent) => void
  onAppendDragEnd: (event: DragEndEvent) => void
}

export interface SortableProxySectionProps {
  kind: ProxySequenceKind
  items: IProxyConfig[]
  onDelete: (name: string) => void
  onDragEnd: (event: DragEndEvent) => void
}

export interface ProxiesYamlEditorPaneProps {
  currData: string
  onChange: (value: string) => void
  onMount: (editorInstance: MonacoEditorInstance) => void
}
