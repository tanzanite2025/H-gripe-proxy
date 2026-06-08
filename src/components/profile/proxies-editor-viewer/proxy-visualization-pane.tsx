import type { ChangeEvent, ReactNode } from 'react'

import { BaseSearchBox, VirtualList } from '@/components/base'
import { Button } from '@/components/tailwind/Button'
import { List, ListItem } from '@/components/tailwind/List'
import { TextField } from '@/components/tailwind/TextField'

import { ProxyItem } from '../proxy-item'
import { SortableProxySection } from './sortable-proxy-section'
import type {
  ProxyVisualizationPaneProps,
  ProxyVisualizationSection,
} from './types'

const getSectionKey = (
  section: ProxyVisualizationSection,
  index: number,
) => {
  if (section.kind === 'original') {
    return `original-${section.proxy.name}`
  }

  return `${section.kind}-${index}`
}

const ArrowUpIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
    <path d="M8 11h3v10h2V11h3l-4-4-4 4zM4 3v2h16V3H4z" />
  </svg>
)

const ArrowDownIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
    <path d="M16 13h-3V3h-2v10H8l4 4 4-4zM4 19v2h16v-2H4z" />
  </svg>
)

export function ProxyVisualizationPane({
  proxyUri,
  sections,
  importPlaceholder,
  prependLabel,
  appendLabel,
  onProxyUriChange,
  onSearchChange,
  onPrependImport,
  onAppendImport,
  onPrependDelete,
  onAppendDelete,
  onOriginalDeleteToggle,
  onPrependDragEnd,
  onAppendDragEnd,
}: ProxyVisualizationPaneProps) {
  const renderSection = (index: number): ReactNode => {
    const section = sections[index]
    if (!section) {
      return null
    }

    if (section.kind === 'prepend') {
      return (
        <SortableProxySection
          kind="prepend"
          items={section.items}
          onDelete={onPrependDelete}
          onDragEnd={onPrependDragEnd}
        />
      )
    }

    if (section.kind === 'append') {
      return (
        <SortableProxySection
          kind="append"
          items={section.items}
          onDelete={onAppendDelete}
          onDragEnd={onAppendDragEnd}
        />
      )
    }

    return (
      <ProxyItem
        key={section.proxy.name}
        type={section.deleted ? 'delete' : 'original'}
        proxy={section.proxy}
        onDelete={() => onOriginalDeleteToggle(section.proxy.name)}
      />
    )
  }

  return (
    <>
      <List className="w-1/2 px-2.5">
        <div className="h-[calc(100%-80px)] overflow-y-auto">
          <ListItem className="py-1.5 px-0.5">
            <TextField
              autoComplete="new-password"
              placeholder={importPlaceholder}
              className="w-full"
              rows={9}
              multiline
              value={proxyUri}
              onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
                onProxyUriChange(event.target.value)
              }
            />
          </ListItem>
        </div>
        <ListItem className="py-1.5 px-0.5">
          <Button
            className="w-full"
            variant="primary"
            startIcon={<ArrowUpIcon />}
            onClick={onPrependImport}
          >
            {prependLabel}
          </Button>
        </ListItem>
        <ListItem className="py-1.5 px-0.5">
          <Button
            className="w-full"
            variant="primary"
            startIcon={<ArrowDownIcon />}
            onClick={onAppendImport}
          >
            {appendLabel}
          </Button>
        </ListItem>
      </List>

      <List className="w-1/2 px-2.5">
        <BaseSearchBox onSearch={onSearchChange} />
        <VirtualList
          count={sections.length}
          estimateSize={56}
          getItemKey={(index) => getSectionKey(sections[index], index)}
          renderItem={renderSection}
          style={{ height: 'calc(100% - 24px)', marginTop: '8px' }}
        />
      </List>
    </>
  )
}
