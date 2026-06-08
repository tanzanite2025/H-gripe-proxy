export interface SpeedTestTransferMetrics {
  speed: number
  duration: number
  dataSize: number
  stability: number
  samples: number[]
}

export interface SpeedTestLatencyMetrics {
  min: number
  max: number
  avg: number
  jitter: number
  samples: number[]
}

export interface SpeedTestPacketLossMetrics {
  sent: number
  received: number
  lossRate: number
}

export interface SpeedTestResult {
  download: SpeedTestTransferMetrics
  upload: SpeedTestTransferMetrics
  latency: SpeedTestLatencyMetrics
  packetLoss: SpeedTestPacketLossMetrics
  timestamp: number
  error?: string
}

export type SpeedTestPhase =
  | 'download'
  | 'upload'
  | 'latency'
  | 'packet-loss'
  | 'complete'

export interface SpeedTestProgress {
  phase: SpeedTestPhase
  progress: number
  currentSpeed?: number
  message?: string
}

export type SpeedTestProgressCallback = (progress: SpeedTestProgress) => void

export interface MetricGrade {
  grade: 'excellent' | 'good' | 'fair' | 'poor'
  label: string
  color: string
}
