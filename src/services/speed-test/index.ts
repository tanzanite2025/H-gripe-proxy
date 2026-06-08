export {
  formatDataSize,
  formatLatency,
  formatSpeed,
  getLatencyGrade,
  getPacketLossGrade,
  getSpeedGrade,
  getSpeedTestPhaseLabel,
} from './formatters'
export { SpeedTestService } from './service'
export type {
  MetricGrade,
  SpeedTestLatencyMetrics,
  SpeedTestPacketLossMetrics,
  SpeedTestPhase,
  SpeedTestProgress,
  SpeedTestProgressCallback,
  SpeedTestResult,
  SpeedTestTransferMetrics,
} from './types'
