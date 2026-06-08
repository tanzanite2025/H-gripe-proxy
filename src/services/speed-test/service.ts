import { fetch } from '@tauri-apps/plugin-http'
import { extractErrorMessage } from 'foxts/extract-error-message'

import { debugLog } from '@/utils/misc'

import {
  buildDownloadEndpoint,
  SPEED_TEST_ENDPOINTS,
  SPEED_TEST_SETTINGS,
} from './constants'
import { calculateJitter, calculateStability } from './metrics'
import type {
  SpeedTestProgress,
  SpeedTestProgressCallback,
  SpeedTestResult,
} from './types'

const sleep = (durationMs: number) =>
  new Promise((resolve) => setTimeout(resolve, durationMs))

const toMbps = (bytes: number, durationMs: number) =>
  (bytes * 8) / (durationMs / 1000) / 1_000_000

const toMegabytes = (bytes: number) => bytes / 1024 / 1024

export class SpeedTestService {
  private abortController: AbortController | null = null
  private onProgress?: SpeedTestProgressCallback

  constructor(onProgress?: SpeedTestProgressCallback) {
    this.onProgress = onProgress
  }

  async runFullTest(): Promise<SpeedTestResult> {
    this.abortController = new AbortController()

    try {
      debugLog('[SpeedTest] Starting full test')

      this.reportProgress({
        phase: 'download',
        progress: 0,
        message: '准备下载测速...',
      })
      const download = await this.testDownloadSpeed()

      this.reportProgress({
        phase: 'upload',
        progress: 0,
        message: '准备上传测速...',
      })
      const upload = await this.testUploadSpeed()

      this.reportProgress({
        phase: 'latency',
        progress: 0,
        message: '测试延迟...',
      })
      const latency = await this.testLatency()

      this.reportProgress({
        phase: 'packet-loss',
        progress: 0,
        message: '测试丢包...',
      })
      const packetLoss = await this.testPacketLoss()

      this.reportProgress({
        phase: 'complete',
        progress: 100,
        message: '测试完成',
      })

      const result: SpeedTestResult = {
        download,
        upload,
        latency,
        packetLoss,
        timestamp: Date.now(),
      }

      debugLog('[SpeedTest] Test completed', result)

      return result
    } catch (error) {
      debugLog('[SpeedTest] Test failed', error)
      throw this.wrapError(error, '测速失败')
    } finally {
      this.abortController = null
    }
  }

  async testDownloadSpeed(): Promise<SpeedTestResult['download']> {
    try {
      const testUrl = buildDownloadEndpoint(SPEED_TEST_SETTINGS.downloadBytes)
      const startTime = performance.now()
      let downloadedBytes = 0
      let lastSampleTime = startTime
      const samples: number[] = []

      const response = await fetch(testUrl, {
        method: 'GET',
        signal: this.abortController?.signal,
        connectTimeout: SPEED_TEST_SETTINGS.downloadConnectTimeoutMs,
      })

      if (!response.ok) {
        throw new Error(`下载测速失败: ${response.status}`)
      }

      if (!response.body) {
        throw new Error('下载测速失败: 响应体不可用')
      }

      const reader = response.body.getReader()

      while (true) {
        const { done, value } = await reader.read()
        if (done) {
          break
        }

        downloadedBytes += value.length

        const now = performance.now()
        if (
          now - lastSampleTime >= SPEED_TEST_SETTINGS.downloadSampleIntervalMs
        ) {
          const elapsedMs = Math.max(now - startTime, 1)
          const speed = toMbps(downloadedBytes, elapsedMs)
          samples.push(speed)

          this.reportProgress({
            phase: 'download',
            progress: Math.min(
              (downloadedBytes / SPEED_TEST_SETTINGS.downloadBytes) * 100,
              100,
            ),
            currentSpeed: speed,
            message: `下载中... ${speed.toFixed(2)} Mbps`,
          })

          lastSampleTime = now
        }
      }

      const duration = Math.max(performance.now() - startTime, 1)
      const speed = toMbps(downloadedBytes, duration)

      return {
        speed,
        duration,
        dataSize: toMegabytes(downloadedBytes),
        stability: calculateStability(samples),
        samples,
      }
    } catch (error) {
      debugLog('[SpeedTest] Download test failed', error)
      throw this.wrapError(error, '下载测速失败')
    }
  }

  async testUploadSpeed(): Promise<SpeedTestResult['upload']> {
    try {
      const testData = this.createUploadPayload()
      const uploadBody = new Blob([testData], {
        type: 'application/octet-stream',
      })
      const startTime = performance.now()

      const response = await fetch(SPEED_TEST_ENDPOINTS.upload, {
        method: 'POST',
        body: uploadBody,
        signal: this.abortController?.signal,
        connectTimeout: SPEED_TEST_SETTINGS.uploadConnectTimeoutMs,
        headers: {
          'Content-Type': 'application/octet-stream',
        },
      })

      if (!response.ok) {
        throw new Error(`上传测速失败: ${response.status}`)
      }

      const duration = Math.max(performance.now() - startTime, 1)
      const speed = toMbps(SPEED_TEST_SETTINGS.uploadBytes, duration)
      const samples = [speed]

      this.reportProgress({
        phase: 'upload',
        progress: 100,
        currentSpeed: speed,
        message: `上传完成 ${speed.toFixed(2)} Mbps`,
      })

      return {
        speed,
        duration,
        dataSize: toMegabytes(SPEED_TEST_SETTINGS.uploadBytes),
        stability: 100,
        samples,
      }
    } catch (error) {
      debugLog('[SpeedTest] Upload test failed', error)
      throw this.wrapError(error, '上传测速失败')
    }
  }

  async testLatency(): Promise<SpeedTestResult['latency']> {
    try {
      const testUrl = buildDownloadEndpoint(1)
      const samples: number[] = []

      for (let attempt = 0; attempt < SPEED_TEST_SETTINGS.latencyAttempts; attempt += 1) {
        const startTime = performance.now()

        const response = await fetch(testUrl, {
          method: 'GET',
          signal: this.abortController?.signal,
          connectTimeout: SPEED_TEST_SETTINGS.latencyConnectTimeoutMs,
        })

        if (response.ok) {
          await response.arrayBuffer()
        }

        samples.push(performance.now() - startTime)

        this.reportProgress({
          phase: 'latency',
          progress:
            ((attempt + 1) / SPEED_TEST_SETTINGS.latencyAttempts) * 100,
          message: `延迟测试 ${attempt + 1}/${SPEED_TEST_SETTINGS.latencyAttempts}`,
        })

        if (attempt < SPEED_TEST_SETTINGS.latencyAttempts - 1) {
          await sleep(SPEED_TEST_SETTINGS.latencyDelayMs)
        }
      }

      const min = Math.min(...samples)
      const max = Math.max(...samples)
      const avg = samples.reduce((sum, value) => sum + value, 0) / samples.length

      return {
        min,
        max,
        avg,
        jitter: calculateJitter(samples),
        samples,
      }
    } catch (error) {
      debugLog('[SpeedTest] Latency test failed', error)
      throw this.wrapError(error, '延迟测试失败')
    }
  }

  async testPacketLoss(): Promise<SpeedTestResult['packetLoss']> {
    try {
      const testUrl = buildDownloadEndpoint(1)
      let received = 0

      for (
        let attempt = 0;
        attempt < SPEED_TEST_SETTINGS.packetLossAttempts;
        attempt += 1
      ) {
        try {
          const response = await fetch(testUrl, {
            method: 'GET',
            signal: this.abortController?.signal,
            connectTimeout: SPEED_TEST_SETTINGS.packetLossConnectTimeoutMs,
          })

          if (response.ok) {
            received += 1
          }
        } catch (error) {
          debugLog('[SpeedTest] Packet loss probe failed', error)
        }

        this.reportProgress({
          phase: 'packet-loss',
          progress:
            ((attempt + 1) / SPEED_TEST_SETTINGS.packetLossAttempts) * 100,
          message: `丢包测试 ${attempt + 1}/${SPEED_TEST_SETTINGS.packetLossAttempts}`,
        })

        if (attempt < SPEED_TEST_SETTINGS.packetLossAttempts - 1) {
          await sleep(SPEED_TEST_SETTINGS.packetLossDelayMs)
        }
      }

      return {
        sent: SPEED_TEST_SETTINGS.packetLossAttempts,
        received,
        lossRate:
          ((SPEED_TEST_SETTINGS.packetLossAttempts - received) /
            SPEED_TEST_SETTINGS.packetLossAttempts) *
          100,
      }
    } catch (error) {
      debugLog('[SpeedTest] Packet loss test failed', error)
      throw this.wrapError(error, '丢包测试失败')
    }
  }

  abort(): void {
    if (!this.abortController) {
      return
    }

    this.abortController.abort()
    debugLog('[SpeedTest] Test aborted')
  }

  private createUploadPayload(): Uint8Array<ArrayBuffer> {
    const payload = new Uint8Array(
      new ArrayBuffer(SPEED_TEST_SETTINGS.uploadBytes),
    )

    for (let index = 0; index < payload.length; index += 1024) {
      payload[index] = Math.floor(Math.random() * 256)
    }

    return payload
  }

  private reportProgress(progress: SpeedTestProgress): void {
    this.onProgress?.(progress)
  }

  private wrapError(error: unknown, fallbackMessage: string): Error {
    return new Error(extractErrorMessage(error) || fallbackMessage, {
      cause: error,
    })
  }
}
