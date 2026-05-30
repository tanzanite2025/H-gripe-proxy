/**
 * 网络诊断页面
 * 集中所有网络检测功能，提供专业的网络诊断工具
 */

import { Shield } from 'lucide-react'
import { Suspense, lazy } from 'react'
import { useTranslation } from 'react-i18next'

import { BasePage } from '@/components/base'
import { EnhancedCard } from '@/components/home/enhanced-card'
import { TorConfigCard } from '@/components/setting/tor-config-card'
import { Grid, Skeleton } from '@/components/tailwind'

// 懒加载所有诊断卡片
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
  useTranslation()

  return (
    <BasePage
      title="网络诊断"
      contentStyle={{ padding: 2 }}
    >
      <Grid container spacing={2} columns={{ xs: 6, sm: 6, md: 12 }} className="items-start">
        {/* WebRTC 泄漏 */}
        <Grid size={6}>
          <Suspense fallback={<Skeleton variant="rectangular" height={250} />}>
            <LazyWebRTCLeakCard />
          </Suspense>
        </Grid>

        {/* Tor 代理 */}
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

        {/* 速度测试（全宽） */}
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
