import { Shield } from 'lucide-react'
import { Suspense, lazy } from 'react'

import { BasePage } from '@/components/base'
import { EnhancedCard } from '@/components/home/enhanced-card'
import { TorConfigCard } from '@/components/setting/tor-config-card'
import { Grid, Skeleton } from '@/components/tailwind'

const LazyWebRTCLeakCard = lazy(() =>
  import('@/components/home/webrtc-leak-card').then((module) => ({
    default: module.WebRTCLeakCard,
  })),
)

const LazySpeedTestCard = lazy(() =>
  import('@/components/home/speed-test-card').then((module) => ({
    default: module.SpeedTestCard,
  })),
)

const NetworkDiagnosticPage = () => {
  return (
    <BasePage title="网络诊断" contentStyle={{ padding: 2 }}>
      <Grid
        container
        spacing={2}
        columns={{ xs: 6, sm: 6, md: 12 }}
        className="items-start"
      >
        <Grid size={6}>
          <Suspense fallback={<Skeleton variant="rectangular" height={250} />}>
            <LazyWebRTCLeakCard />
          </Suspense>
        </Grid>

        <Grid size={6}>
          <EnhancedCard
            title="Tor 代理"
            icon={<Shield className="h-5 w-5" />}
            iconColor="warning"
            fixedHeight={280}
          >
            <TorConfigCard />
          </EnhancedCard>
        </Grid>

        <Grid size={12}>
          <Suspense fallback={<Skeleton variant="rectangular" height={350} />}>
            <LazySpeedTestCard />
          </Suspense>
        </Grid>
      </Grid>
    </BasePage>
  )
}

export default NetworkDiagnosticPage
