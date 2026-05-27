import { cn } from '@/utils/cn'
import type { ReactNode } from 'react'

import type { SearchState } from '@/components/base'

interface Props {
  value: ILogItem
  searchState?: SearchState
}

const LogItem = ({ value, searchState }: Props) => {
  const renderHighlightText = (text: string) => {
    if (!searchState?.text.trim()) return text

    try {
      const searchText = searchState.text
      let pattern: string

      if (searchState.useRegularExpression) {
        try {
          new RegExp(searchText)
          pattern = searchText
        } catch {
          pattern = searchText.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
        }
      } else {
        const escaped = searchText.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
        pattern = searchState.matchWholeWord ? `\\b${escaped}\\b` : escaped
      }

      const flags = searchState.matchCase ? 'g' : 'gi'
      const regex = new RegExp(pattern, flags)
      const elements: ReactNode[] = []
      let lastIndex = 0
      let match: RegExpExecArray | null

      while ((match = regex.exec(text)) !== null) {
        const start = match.index
        const matchText = match[0]

        if (matchText === '') {
          regex.lastIndex += 1
          continue
        }

        if (start > lastIndex) {
          elements.push(text.slice(lastIndex, start))
        }

        elements.push(
          <span key={`highlight-${start}`} className="rounded bg-yellow-400/40 px-0.5 dark:bg-yellow-400/25">
            {matchText}
          </span>,
        )

        lastIndex = start + matchText.length
      }

      if (lastIndex < text.length) {
        elements.push(text.slice(lastIndex))
      }

      return elements.length ? elements : text
    } catch {
      return text
    }
  }

  const typeClass = cn(
    'ml-2 inline-block rounded text-center font-semibold uppercase',
    value.type.toLowerCase() === 'error' || value.type.toLowerCase() === 'err'
      ? 'text-red-500 dark:text-red-400'
      : value.type.toLowerCase() === 'warning' || value.type.toLowerCase() === 'warn'
        ? 'text-yellow-500 dark:text-yellow-400'
        : value.type.toLowerCase() === 'info' || value.type.toLowerCase() === 'inf'
          ? 'text-blue-500 dark:text-blue-400'
          : ''
  )

  return (
    <div className="mx-3 select-text border-b border-divider py-2 text-sm leading-tight">
      <div>
        <span className="text-gray-600 dark:text-gray-400">
          {renderHighlightText(value.time || '')}
        </span>
        <span className={typeClass}>
          {renderHighlightText(value.type)}
        </span>
      </div>
      <div>
        <span className="break-anywhere text-gray-900 dark:text-gray-100">
          {renderHighlightText(value.payload)}
        </span>
      </div>
    </div>
  )
}

export default LogItem
