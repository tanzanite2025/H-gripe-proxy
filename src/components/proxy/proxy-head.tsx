import {
  Clock,
  MapPin,
  Wifi,
  Filter,
  FilterX,
  Eye,
  EyeOff,
  WifiOff,
  SortAsc,
  ArrowUpDown,
} from 'lucide-react'
import { useEffect, useState, type ChangeEvent } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseSearchBox } from '@/components/base'
import { IconButton, TextField } from '@/components/tailwind'
import { useVerge } from '@/hooks/system'
import delayManager from '@/services/delay'
import { debugLog } from '@/utils/misc'

import type { ProxySortType } from './use-filter-sort'
import type { HeadState } from './use-head-state'

interface Props {
  className?: string
  url?: string
  groupName: string
  headState: HeadState
  onLocation: () => void
  onCheckDelay: () => void
  onHeadState: (val: Partial<HeadState>) => void
}

export const ProxyHead = ({
  className = '',
  url,
  groupName,
  headState,
  onHeadState,
  onLocation,
  onCheckDelay,
}: Props) => {
  const {
    showType,
    sortType,
    filterText,
    textState,
    testUrl,
    filterMatchCase,
    filterMatchWholeWord,
    filterUseRegularExpression,
  } = headState

  const { t } = useTranslation()
  const [autoFocus, setAutoFocus] = useState(false)

  useEffect(() => {
    // fix the focus conflict
    const timer = setTimeout(() => setAutoFocus(true), 100)
    return () => clearTimeout(timer)
  }, [])

  const { verge } = useVerge()
  const defaultLatencyUrl =
    verge?.default_latency_test?.trim() ||
    'https://cp.cloudflare.com/generate_204'

  useEffect(() => {
    delayManager.setUrl(groupName, testUrl?.trim() || url || defaultLatencyUrl)
  }, [groupName, testUrl, defaultLatencyUrl, url])

  return (
    <div className={`flex items-center gap-1 ${className}`}>
      <IconButton
        size="small"
        title={t('proxies.page.tooltips.locate')}
        onClick={onLocation}
      >
        <MapPin className="h-4 w-4" />
      </IconButton>

      <IconButton
        size="small"
        title={t('proxies.page.tooltips.delayCheck')}
        onClick={() => {
          debugLog(`[ProxyHead] 点击延迟测试按钮，组: ${groupName}`)
          // Remind the user that it is custom test url
          if (testUrl?.trim() && textState !== 'filter') {
            debugLog(`[ProxyHead] 使用自定义测试URL: ${testUrl}`)
            onHeadState({ textState: 'url' })
          }
          onCheckDelay()
        }}
      >
        <Wifi className="h-4 w-4" />
      </IconButton>

      <IconButton
        size="small"
        title={
          [
            t('proxies.page.tooltips.sortDefault'),
            t('proxies.page.tooltips.sortDelay'),
            t('proxies.page.tooltips.sortName'),
          ][sortType]
        }
        onClick={() =>
          onHeadState({ sortType: ((sortType + 1) % 3) as ProxySortType })
        }
      >
        {sortType !== 1 && sortType !== 2 && <ArrowUpDown className="h-4 w-4" />}
        {sortType === 1 && <Clock className="h-4 w-4" />}
        {sortType === 2 && <SortAsc className="h-4 w-4" />}
      </IconButton>

      <IconButton
        size="small"
        title={t('proxies.page.tooltips.delayCheckUrl')}
        onClick={() =>
          onHeadState({ textState: textState === 'url' ? null : 'url' })
        }
      >
        {textState === 'url' ? (
          <Wifi className="h-4 w-4" />
        ) : (
          <WifiOff className="h-4 w-4" />
        )}
      </IconButton>

      <IconButton
        size="small"
        title={
          showType
            ? t('proxies.page.tooltips.showBasic')
            : t('proxies.page.tooltips.showDetail')
        }
        onClick={() => onHeadState({ showType: !showType })}
      >
        {showType ? <Eye className="h-4 w-4" /> : <EyeOff className="h-4 w-4" />}
      </IconButton>

      <IconButton
        size="small"
        title={t('proxies.page.tooltips.filter')}
        onClick={() =>
          onHeadState({ textState: textState === 'filter' ? null : 'filter' })
        }
      >
        {textState === 'filter' ? (
          <Filter className="h-4 w-4" />
        ) : (
          <FilterX className="h-4 w-4" />
        )}
      </IconButton>

      {textState === 'filter' && (
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
      )}

      {textState === 'url' && (
        <TextField
          autoComplete="new-password"
          autoFocus={autoFocus}
          autoSave="off"
          value={testUrl}
          placeholder={t('proxies.page.placeholders.delayCheckUrl')}
          onChange={(e: ChangeEvent<HTMLInputElement>) => onHeadState({ testUrl: e.target.value })}
          className="ml-1 flex-1"
          inputClassName="py-1.5 px-2"
        />
      )}
    </div>
  )
}
