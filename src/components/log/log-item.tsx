import type { ReactNode } from 'react'

import type { SearchState } from '@/components/base'
import { cn } from '@/utils/cn'

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
    'inline-flex w-fit items-center rounded-full px-2.5 py-1 text-[11px] font-semibold uppercase tracking-[0.16em]',
    value.type.toLowerCase() === 'error' || value.type.toLowerCase() === 'err'
      ? 'bg-red-500/10 text-red-500 dark:bg-red-500/15 dark:text-red-400'
      : value.type.toLowerCase() === 'warning' || value.type.toLowerCase() === 'warn'
        ? 'bg-yellow-500/10 text-yellow-500 dark:bg-yellow-500/15 dark:text-yellow-400'
        : value.type.toLowerCase() === 'info' || value.type.toLowerCase() === 'inf'
          ? 'bg-teal-500/10 text-teal-500 dark:bg-teal-500/15 dark:text-teal-400'
          : 'bg-white/5 text-text-secondary',
  )

  return (
    <div className="select-text border-b border-divider/70 px-4 py-3 text-sm transition-colors duration-200 hover:bg-white/[0.02]">
      <div className="grid gap-2 md:grid-cols-[132px_88px_minmax(0,1fr)] md:items-start">
        <span className="font-mono text-xs text-text-secondary/80">
          {renderHighlightText(value.time || '')}
        </span>

        <span className={typeClass}>
          {renderHighlightText(value.type)}
        </span>

        <span className="break-anywhere text-sm leading-6 text-text-primary">
          {renderHighlightText(value.payload)}
        </span>
      </div>
    </div>
  )
}

export default LogItem
