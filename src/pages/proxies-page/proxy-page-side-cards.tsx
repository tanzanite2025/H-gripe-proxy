import { Suspense, lazy } from 'react'

import { Skeleton } from '@/components/tailwind'

const LazyProxyDetectionCard = lazy(() =>
  import('@/components/home/proxy-detection-card').then((module) => ({
    default: module.ProxyDetectionCard,
  })),
)

const LazyDNSLeakCard = lazy(() =>
  import('@/components/home/dns-leak-card').then((module) => ({
    default: module.DNSLeakCard,
  })),
)

export const ProxyPageSideCards = () => {
  return (
    <div className="flex h-full min-h-0 flex-col gap-4 overflow-y-auto pb-4 pr-2">
      <Suspense fallback={<Skeleton variant="rectangular" height={200} />}>
        <LazyProxyDetectionCard />
      </Suspense>
      <Suspense fallback={<Skeleton variant="rectangular" height={200} />}>
        <LazyDNSLeakCard />
      </Suspense>
    </div>
  )
}
