import type { ChangeEvent } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseSearchBox } from '@/components/base'
import { TextField } from '@/components/tailwind'

import type { HeadState } from '../use-head-state'

interface ProxyHeadInputProps {
  autoFocus: boolean
  headState: HeadState
  onHeadState: (val: Partial<HeadState>) => void
}

export function ProxyHeadInput({
  autoFocus,
  headState,
  onHeadState,
}: ProxyHeadInputProps) {
  const { t } = useTranslation()
  const {
    filterMatchCase,
    filterMatchWholeWord,
    filterText,
    filterUseRegularExpression,
    testUrl,
    textState,
  } = headState

  if (textState === 'filter') {
    return (
      <div className="ml-1 flex-1">
        <BaseSearchBox
          autoFocus={autoFocus}
          value={filterText}
          searchState={{
            matchCase: filterMatchCase,
            matchWholeWord: filterMatchWholeWord,
            useRegularExpression: filterUseRegularExpression,
          }}
          onSearch={(_, state) =>
            onHeadState({
              filterText: state.text,
              filterMatchCase: state.matchCase,
              filterMatchWholeWord: state.matchWholeWord,
              filterUseRegularExpression: state.useRegularExpression,
            })
          }
        />
      </div>
    )
  }

  if (textState === 'url') {
    return (
      <TextField
        autoComplete="new-password"
        autoFocus={autoFocus}
        autoSave="off"
        value={testUrl}
        placeholder={t('proxies.page.placeholders.delayCheckUrl')}
        onChange={(event: ChangeEvent<HTMLInputElement>) =>
          onHeadState({ testUrl: event.target.value })
        }
        className="ml-1 flex-1"
        inputClassName="px-2 py-1.5"
      />
    )
  }

  return null
}
