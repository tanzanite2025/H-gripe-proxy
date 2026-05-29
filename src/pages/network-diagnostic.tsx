/**
 * 网络诊断页面
 * 集中所有网络检测功能，提供专业的网络诊断工具
 */

import { Suspense, lazy } from 'react'
import { useTranslation } from 'react-i18next'

import { BasePage } from '@/components/base'
import { Grid, Skeleton } from '@/components/tailwind'

// 懒加载所有诊断卡片
const LazyIpInfoCard = lazy(() =>
  import('@/components/home/ip-info-card').then((module) => ({
    default: module.IpInfoCard,
  })),
)

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
      <Grid container spacing={2} columns={{ xs: 6, sm: 6, md: 12 }}>
        {/* 第一行：IP 信息和代理检测 */}
        <Grid size={6}>
          <Suspense fallback={<Skeleton variant="rectangular" height={250} />}>
            <LazyIpInfoCard />
          </Suspense>
        </Grid>
        <Grid size={6}>
          <Suspense fallback={<Skeleton variant="rectangular" height={250} />}>
            <LazyProxyDetectionCard />
          </Suspense>
        </Grid>

        {/* 第二行：DNS 泄漏和 WebRTC 泄漏 */}
        <Grid size={6}>
          <Suspense fallback={<Skeleton variant="rectangular" height={250} />}>
            <LazyDNSLeakCard />
          </Suspense>
        </Grid>
        <Grid size={6}>
          <Suspense fallback={<Skeleton variant="rectangular" height={250} />}>
            <LazyWebRTCLeakCard />
          </Suspense>
        </Grid>

        {/* 第三行：速度测试（全宽） */}
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
