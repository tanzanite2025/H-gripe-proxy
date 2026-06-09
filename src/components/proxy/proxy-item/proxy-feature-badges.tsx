import { Tooltip } from '@/components/tailwind/Tooltip'

import {
  getMieruMultiplexShortText,
  getMieruMultiplexTooltip,
  getSmuxShortText,
  getSmuxTooltip,
  getSudokuMultiplexShortText,
  getSudokuMultiplexTooltip,
} from '../utils/multiplexing-helpers'

interface ProxyFeatureBadgesProps {
  proxy: IProxyItem
  badgeClassName: string
}

interface BadgeItem {
  key: string
  label: string
  tooltip?: string
}

const renderBadge = (badge: BadgeItem, badgeClassName: string) => {
  const content = <span className={badgeClassName}>{badge.label}</span>

  if (!badge.tooltip) {
    return <span key={badge.key}>{content}</span>
  }

  return (
    <Tooltip key={badge.key} title={badge.tooltip} arrow placement="top">
      {content}
    </Tooltip>
  )
}

const buildProxyFeatureBadges = (proxy: IProxyItem): BadgeItem[] => {
  const badges: BadgeItem[] = []

  if (proxy.provider) {
    badges.push({
      key: 'provider',
      label: proxy.provider,
    })
  }

  badges.push({
    key: 'type',
    label: proxy.type,
  })

  if (proxy.udp) {
    badges.push({ key: 'udp', label: 'UDP' })
  }

  if (proxy.xudp) {
    badges.push({ key: 'xudp', label: 'XUDP' })
  }

  if (proxy.tfo) {
    badges.push({ key: 'tfo', label: 'TFO' })
  }

  if (proxy.mptcp) {
    badges.push({ key: 'mptcp', label: 'MPTCP' })
  }

  if (proxy.smux) {
    badges.push({
      key: 'smux',
      label: getSmuxShortText(proxy),
      tooltip: getSmuxTooltip(proxy),
    })
  }

  if (
    proxy.type === 'mieru' &&
    (proxy as any).multiplexing &&
    (proxy as any).multiplexing !== 'MULTIPLEXING_OFF'
  ) {
    badges.push({
      key: 'mieru-mux',
      label: getMieruMultiplexShortText((proxy as any).multiplexing),
      tooltip: getMieruMultiplexTooltip((proxy as any).multiplexing),
    })
  }

  if (
    proxy.type === 'sudoku' &&
    (proxy as any).httpmask?.multiplex &&
    (proxy as any).httpmask.multiplex !== 'off'
  ) {
    badges.push({
      key: 'sudoku-httpmask',
      label: getSudokuMultiplexShortText((proxy as any).httpmask.multiplex),
      tooltip: getSudokuMultiplexTooltip((proxy as any).httpmask.multiplex),
    })
  }

  return badges
}

export function ProxyFeatureBadges({
  proxy,
  badgeClassName,
}: ProxyFeatureBadgesProps) {
  return buildProxyFeatureBadges(proxy).map((badge) =>
    renderBadge(badge, badgeClassName),
  )
}
