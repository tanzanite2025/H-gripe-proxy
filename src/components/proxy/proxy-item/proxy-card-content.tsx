import { ProxyFeatureBadges } from './proxy-feature-badges'

type ProxyCardContentVariant = 'default' | 'compact'

interface ProxyCardContentProps {
  proxy: IProxyItem
  showType: boolean
  variant: ProxyCardContentVariant
}

const BADGE_CLASSNAME: Record<ProxyCardContentVariant, string> = {
  default:
    'mr-1 inline-block rounded border border-gray-400/40 px-0.5 text-[10px] leading-5 text-gray-400/50',
  compact:
    'mr-1 inline-block rounded border border-text-secondary px-1 text-[10px] leading-normal text-text-secondary',
}

export function ProxyCardContent({
  proxy,
  showType,
  variant,
}: ProxyCardContentProps) {
  if (variant === 'default') {
    return (
      <>
        <div className="mr-2 inline-block text-sm text-current">
          {proxy.name}
          {showType && proxy.now && ` - ${proxy.now}`}
        </div>
        {showType && (
          <ProxyFeatureBadges
            proxy={proxy}
            badgeClassName={BADGE_CLASSNAME.default}
          />
        )}
      </>
    )
  }

  return (
    <div title={`${proxy.name}\n${proxy.now ?? ''}`} className="overflow-hidden">
      <div className="block overflow-hidden text-ellipsis whitespace-nowrap break-all text-sm text-text-primary">
        {proxy.name}
      </div>

      {showType && (
        <div className="mt-1 flex flex-none flex-nowrap">
          {proxy.now && (
            <div className="mr-2 block overflow-hidden text-ellipsis whitespace-nowrap break-all text-sm text-text-secondary">
              {proxy.now}
            </div>
          )}
          <ProxyFeatureBadges
            proxy={proxy}
            badgeClassName={BADGE_CLASSNAME.compact}
          />
        </div>
      )}
    </div>
  )
}
