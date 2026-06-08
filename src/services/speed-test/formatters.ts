import type { MetricGrade, SpeedTestPhase } from './types'

const SPEED_TEST_PHASE_LABELS: Record<SpeedTestPhase, string> = {
  download: '下载速度测试',
  upload: '上传速度测试',
  latency: '延迟测试',
  'packet-loss': '丢包测试',
  complete: '测试完成',
}

const GRADE_META: Record<MetricGrade['grade'], Omit<MetricGrade, 'grade'>> = {
  excellent: { label: '优秀', color: 'text-success' },
  good: { label: '良好', color: 'text-info' },
  fair: { label: '一般', color: 'text-warning' },
  poor: { label: '较差', color: 'text-error' },
}

const createGrade = (grade: MetricGrade['grade']): MetricGrade => ({
  grade,
  ...GRADE_META[grade],
})

export function getSpeedTestPhaseLabel(phase: SpeedTestPhase): string {
  return SPEED_TEST_PHASE_LABELS[phase]
}

export function formatSpeed(mbps: number): string {
  if (mbps >= 1000) {
    return `${(mbps / 1000).toFixed(2)} Gbps`
  }

  return `${mbps.toFixed(2)} Mbps`
}

export function formatLatency(ms: number): string {
  if (ms >= 1000) {
    return `${(ms / 1000).toFixed(2)} s`
  }

  return `${Math.round(ms)} ms`
}

export function formatDataSize(mb: number): string {
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(2)} GB`
  }

  return `${mb.toFixed(2)} MB`
}

export function getSpeedGrade(mbps: number): MetricGrade {
  if (mbps >= 100) {
    return createGrade('excellent')
  }

  if (mbps >= 50) {
    return createGrade('good')
  }

  if (mbps >= 10) {
    return createGrade('fair')
  }

  return createGrade('poor')
}

export function getLatencyGrade(ms: number): MetricGrade {
  if (ms <= 30) {
    return createGrade('excellent')
  }

  if (ms <= 100) {
    return createGrade('good')
  }

  if (ms <= 300) {
    return createGrade('fair')
  }

  return createGrade('poor')
}

export function getPacketLossGrade(lossRate: number): MetricGrade {
  if (lossRate <= 0.5) {
    return createGrade('excellent')
  }

  if (lossRate <= 2) {
    return createGrade('good')
  }

  if (lossRate <= 5) {
    return createGrade('fair')
  }

  return createGrade('poor')
}
