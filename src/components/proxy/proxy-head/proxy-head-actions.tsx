import {
  ArrowUpDown,
  Clock,
  Eye,
  EyeOff,
  Filter,
  FilterX,
  MapPin,
  SortAsc,
  Wifi,
  WifiOff,
} from 'lucide-react'
import type { ReactElement } from 'react'
import { useTranslation } from 'react-i18next'

import { IconButton } from '@/components/tailwind'
import { debugLog } from '@/utils/misc'

import type { ProxySortType } from '../use-filter-sort'
import type { HeadState } from '../use-head-state'

interface ProxyHeadActionsProps {
  groupName: string
  headState: HeadState
  onLocation: () => void
  onCheckDelay: () => void
  onHeadState: (val: Partial<HeadState>) => void
}

const SORT_TYPE_ICONS: Record<ProxySortType, ReactElement> = {
  0: <ArrowUpDown className="h-4 w-4" />,
  1: <Clock className="h-4 w-4" />,
  2: <SortAsc className="h-4 w-4" />,
}

const SORT_TOOLTIP_KEYS = {
  0: 'proxies.page.tooltips.sortDefault',
  1: 'proxies.page.tooltips.sortDelay',
  2: 'proxies.page.tooltips.sortName',
} as const

const getNextSortType = (sortType: ProxySortType): ProxySortType =>
  ((sortType + 1) % 3) as ProxySortType

const toggleTextState = (
  current: HeadState['textState'],
  next: Exclude<HeadState['textState'], null>,
): HeadState['textState'] => (current === next ? null : next)

export function ProxyHeadActions({
  groupName,
  headState,
  onLocation,
  onCheckDelay,
  onHeadState,
}: ProxyHeadActionsProps) {
  const { t } = useTranslation()
  const { showType, sortType, textState, testUrl } = headState

  const handleDelayCheck = () => {
    debugLog(`[ProxyHead] Delay check clicked, group: ${groupName}`)

    if (testUrl?.trim() && textState !== 'filter') {
      debugLog(`[ProxyHead] Using custom delay test URL: ${testUrl}`)
      onHeadState({ textState: 'url' })
    }

    onCheckDelay()
  }

  return (
    <>
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
        onClick={handleDelayCheck}
      >
        <Wifi className="h-4 w-4" />
      </IconButton>

      <IconButton
        size="small"
        title={t(SORT_TOOLTIP_KEYS[sortType])}
        onClick={() => onHeadState({ sortType: getNextSortType(sortType) })}
      >
        {SORT_TYPE_ICONS[sortType]}
      </IconButton>

      <IconButton
        size="small"
        title={t('proxies.page.tooltips.delayCheckUrl')}
        onClick={() =>
          onHeadState({ textState: toggleTextState(textState, 'url') })
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
          onHeadState({ textState: toggleTextState(textState, 'filter') })
        }
      >
        {textState === 'filter' ? (
          <Filter className="h-4 w-4" />
        ) : (
          <FilterX className="h-4 w-4" />
        )}
      </IconButton>
    </>
  )
}
