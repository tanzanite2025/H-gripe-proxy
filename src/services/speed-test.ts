/**
 * 网络速度测试服务
 * 测试下载/上传速度、延迟、丢包率、抖动等指标
 */

import { fetch } from '@tauri-apps/plugin-http'
import { extractErrorMessage } from 'foxts/extract-error-message'

import { debugLog } from '@/utils/misc'

export interface SpeedTestResult {
  // 下载测试
  download: {
    speed: number        // Mbps
    duration: number     // ms
    dataSize: number     // MB
    stability: number    // 0-100
    samples: number[]    // 速度采样点
  }
  
  // 上传测试
  upload: {
    speed: number
    duration: number
    dataSize: number
    stability: number
    samples: number[]
  }
  
  // 延迟测试
  latency: {
    min: number          // ms
    max: number
    avg: number
    jitter: number       // 抖动
    samples: number[]
  }
  
  // 丢包测试
  packetLoss: {
    sent: number
    received: number
    lossRate: number     // %
  }
  
  // 测试时间
  timestamp: number
  
  // 错误信息
  error?: string
}

export interface SpeedTestProgress {
  phase: 'download' | 'upload' | 'latency' | 'packet-loss' | 'complete'
  progress: number      // 0-100
  currentSpeed?: number // 当前速度 (Mbps)
  message?: string
}

export type SpeedTestProgressCallback = (progress: SpeedTestProgress) => void

/**
 * 速度测试服务
 */
export class SpeedTestService {
  private abortController: AbortController | null = null
  private onProgress?: SpeedTestProgressCallback
  
  constructor(onProgress?: SpeedTestProgressCallback) {
    this.onProgress = onProgress
  }
  
  /**
   * 运行完整的速度测试
   */
  async runFullTest(): Promise<SpeedTestResult> {
    try {
      this.abortController = new AbortController()
      
      debugLog('[SpeedTest] 开始完整速度测试')
      
      // 1. 下载测试
      this.reportProgress({ phase: 'download', progress: 0, message: '准备下载测试...' })
      const download = await this.testDownloadSpeed()
      
      // 2. 上传测试
      this.reportProgress({ phase: 'upload', progress: 0, message: '准备上传测试...' })
      const upload = await this.testUploadSpeed()
      
      // 3. 延迟测试
      this.reportProgress({ phase: 'latency', progress: 0, message: '测试延迟...' })
      const latency = await this.testLatency()
      
      // 4. 丢包测试
      this.reportProgress({ phase: 'packet-loss', progress: 0, message: '测试丢包率...' })
      const packetLoss = await this.testPacketLoss()
      
      // 5. 完成
      this.reportProgress({ phase: 'complete', progress: 100, message: '测试完成' })
      
      const result: SpeedTestResult = {
        download,
        upload,
        latency,
        packetLoss,
        timestamp: Date.now(),
      }
      
      debugLog('[SpeedTest] 测试完成:', result)
      
      return result
    } catch (error) {
      debugLog('[SpeedTest] 测试失败:', error)
      
      throw new Error(extractErrorMessage(error) || '速度测试失败')
    } finally {
      this.abortController = null
    }
  }
  
  /**
   * 下载速度测试
   */
  async testDownloadSpeed(): Promise<SpeedTestResult['download']> {
    try {
      // 使用 Cloudflare Speed Test 或其他测速服务
      // 测试文件大小：10MB
      const testSize = 10 * 1024 * 1024 // 10MB
      const testUrl = `https://speed.cloudflare.com/__down?bytes=${testSize}`
      
      const startTime = performance.now()
      let downloadedBytes = 0
      const samples: number[] = []
      
      const response = await fetch(testUrl, {
        method: 'GET',
        signal: this.abortController?.signal,
        connectTimeout: 30000,
      })
      
      if (!response.ok) {
        throw new Error(`下载测试失败: ${response.status}`)
      }
      
      const reader = response.body!.getReader()
      let lastSampleTime = startTime
      
      while (true) {
        const { done, value } = await reader.read()
        if (done) break
        
        downloadedBytes += value.length
        
        // 每 200ms 采样一次速度
        const now = performance.now()
        if (now - lastSampleTime >= 200) {
          const elapsed = (now - startTime) / 1000 // 秒
          const speed = (downloadedBytes * 8) / elapsed / 1000000 // Mbps
          samples.push(speed)
          
          // 报告进度
          const progress = Math.min((downloadedBytes / testSize) * 100, 100)
          this.reportProgress({
            phase: 'download',
            progress,
            currentSpeed: speed,
            message: `下载中... ${speed.toFixed(2)} Mbps`,
          })
          
          lastSampleTime = now
        }
      }
      
      const totalTime = performance.now() - startTime
      const avgSpeed = (downloadedBytes * 8) / (totalTime / 1000) / 1000000
      const stability = this.calculateStability(samples)
      
      return {
        speed: avgSpeed,
        duration: totalTime,
        dataSize: downloadedBytes / 1024 / 1024,
        stability,
        samples,
      }
    } catch (error) {
      debugLog('[SpeedTest] 下载测试失败:', error)
      throw error
    }
  }
  
  /**
   * 上传速度测试
   */
  async testUploadSpeed(): Promise<SpeedTestResult['upload']> {
    try {
      // 生成测试数据：5MB
      const testSize = 5 * 1024 * 1024 // 5MB
      const testData = new Uint8Array(testSize)
      
      // 填充随机数据（可选，为了更真实）
      for (let i = 0; i < testSize; i += 1024) {
        testData[i] = Math.floor(Math.random() * 256)
      }
      
      const testUrl = 'https://speed.cloudflare.com/__up'
      
      const startTime = performance.now()
      const samples: number[] = []
      
      // 注意：fetch API 不支持上传进度监听
      // 这里我们只能测试总体上传速度
      const response = await fetch(testUrl, {
        method: 'POST',
        body: testData,
        signal: this.abortController?.signal,
        connectTimeout: 30000,
        headers: {
          'Content-Type': 'application/octet-stream',
        },
      })
      
      if (!response.ok) {
        throw new Error(`上传测试失败: ${response.status}`)
      }
      
      const totalTime = performance.now() - startTime
      const avgSpeed = (testSize * 8) / (totalTime / 1000) / 1000000
      
      // 由于无法获取实时进度，我们只有一个采样点
      samples.push(avgSpeed)
      
      this.reportProgress({
        phase: 'upload',
        progress: 100,
        currentSpeed: avgSpeed,
        message: `上传完成 ${avgSpeed.toFixed(2)} Mbps`,
      })
      
      return {
        speed: avgSpeed,
        duration: totalTime,
        dataSize: testSize / 1024 / 1024,
        stability: 100, // 无法计算稳定性
        samples,
      }
    } catch (error) {
      debugLog('[SpeedTest] 上传测试失败:', error)
      throw error
    }
  }
  
  /**
   * 延迟测试
   */
  async testLatency(): Promise<SpeedTestResult['latency']> {
    try {
      const testUrl = 'https://speed.cloudflare.com/__down?bytes=1'
      const samples: number[] = []
      const testCount = 10
      
      for (let i = 0; i < testCount; i++) {
        const startTime = performance.now()
        
        const response = await fetch(testUrl, {
          method: 'GET',
          signal: this.abortController?.signal,
          connectTimeout: 5000,
        })
        
        if (response.ok) {
          // 读取响应以确保完整的往返时间
          await response.arrayBuffer()
        }
        
        const latency = performance.now() - startTime
        samples.push(latency)
        
        // 报告进度
        this.reportProgress({
          phase: 'latency',
          progress: ((i + 1) / testCount) * 100,
          message: `延迟测试 ${i + 1}/${testCount}`,
        })
        
        // 等待一小段时间再进行下一次测试
        if (i < testCount - 1) {
          await new Promise(resolve => setTimeout(resolve, 100))
        }
      }
      
      const min = Math.min(...samples)
      const max = Math.max(...samples)
      const avg = samples.reduce((a, b) => a + b, 0) / samples.length
      
      // 计算抖动（jitter）- 延迟的标准差
      const jitter = this.calculateJitter(samples)
      
      return {
        min,
        max,
        avg,
        jitter,
        samples,
      }
    } catch (error) {
      debugLog('[SpeedTest] 延迟测试失败:', error)
      throw error
    }
  }
  
  /**
   * 丢包测试
   */
  async testPacketLoss(): Promise<SpeedTestResult['packetLoss']> {
    try {
      const testUrl = 'https://speed.cloudflare.com/__down?bytes=1'
      const testCount = 20
      let received = 0
      
      for (let i = 0; i < testCount; i++) {
        try {
          const response = await fetch(testUrl, {
            method: 'GET',
            signal: this.abortController?.signal,
            connectTimeout: 3000,
          })
          
          if (response.ok) {
            received++
          }
        } catch (error) {
          // 请求失败，视为丢包
          debugLog('[SpeedTest] 丢包:', error)
        }
        
        // 报告进度
        this.reportProgress({
          phase: 'packet-loss',
          progress: ((i + 1) / testCount) * 100,
          message: `丢包测试 ${i + 1}/${testCount}`,
        })
        
        // 等待一小段时间
        if (i < testCount - 1) {
          await new Promise(resolve => setTimeout(resolve, 50))
        }
      }
      
      const lossRate = ((testCount - received) / testCount) * 100
      
      return {
        sent: testCount,
        received,
        lossRate,
      }
    } catch (error) {
      debugLog('[SpeedTest] 丢包测试失败:', error)
      throw error
    }
  }
  
  /**
   * 取消测试
   */
  abort(): void {
    if (this.abortController) {
      this.abortController.abort()
      debugLog('[SpeedTest] 测试已取消')
    }
  }
  
  /**
   * 报告进度
   */
  private reportProgress(progress: SpeedTestProgress): void {
    if (this.onProgress) {
      this.onProgress(progress)
    }
  }
  
  /**
   * 计算稳定性（基于速度方差）
   * 返回 0-100 的分数，100 表示非常稳定
   */
  private calculateStability(samples: number[]): number {
    if (samples.length < 2) return 100
    
    const avg = samples.reduce((a, b) => a + b, 0) / samples.length
    const variance = samples.reduce((sum, val) => sum + Math.pow(val - avg, 2), 0) / samples.length
    const stdDev = Math.sqrt(variance)
    
    // 标准差越小，稳定性越高
    // 将标准差转换为 0-100 的分数
    const coefficientOfVariation = (stdDev / avg) * 100
    const stability = Math.max(0, 100 - coefficientOfVariation)
    
    return Math.round(stability)
  }
  
  /**
   * 计算抖动（延迟的标准差）
   */
  private calculateJitter(samples: number[]): number {
    if (samples.length < 2) return 0
    
    const avg = samples.reduce((a, b) => a + b, 0) / samples.length
    const variance = samples.reduce((sum, val) => sum + Math.pow(val - avg, 2), 0) / samples.length
    const stdDev = Math.sqrt(variance)
    
    return Math.round(stdDev * 100) / 100
  }
}

/**
 * 格式化速度（Mbps）
 */
export function formatSpeed(mbps: number): string {
  if (mbps >= 1000) {
    return `${(mbps / 1000).toFixed(2)} Gbps`
  }
  return `${mbps.toFixed(2)} Mbps`
}

/**
 * 格式化延迟（ms）
 */
export function formatLatency(ms: number): string {
  if (ms >= 1000) {
    return `${(ms / 1000).toFixed(2)} s`
  }
  return `${Math.round(ms)} ms`
}

/**
 * 格式化数据大小（MB）
 */
export function formatDataSize(mb: number): string {
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(2)} GB`
  }
  return `${mb.toFixed(2)} MB`
}

/**
 * 评估速度等级
 */
export function getSpeedGrade(mbps: number): {
  grade: 'excellent' | 'good' | 'fair' | 'poor'
  label: string
  color: string
} {
  if (mbps >= 100) {
    return { grade: 'excellent', label: '优秀', color: 'text-success' }
  } else if (mbps >= 50) {
    return { grade: 'good', label: '良好', color: 'text-info' }
  } else if (mbps >= 10) {
    return { grade: 'fair', label: '一般', color: 'text-warning' }
  } else {
    return { grade: 'poor', label: '较差', color: 'text-error' }
  }
}

/**
 * 评估延迟等级
 */
export function getLatencyGrade(ms: number): {
  grade: 'excellent' | 'good' | 'fair' | 'poor'
  label: string
  color: string
} {
  if (ms <= 30) {
    return { grade: 'excellent', label: '优秀', color: 'text-success' }
  } else if (ms <= 100) {
    return { grade: 'good', label: '良好', color: 'text-info' }
  } else if (ms <= 300) {
    return { grade: 'fair', label: '一般', color: 'text-warning' }
  } else {
    return { grade: 'poor', label: '较差', color: 'text-error' }
  }
}

/**
 * 评估丢包率等级
 */
export function getPacketLossGrade(lossRate: number): {
  grade: 'excellent' | 'good' | 'fair' | 'poor'
  label: string
  color: string
} {
  if (lossRate <= 0.5) {
    return { grade: 'excellent', label: '优秀', color: 'text-success' }
  } else if (lossRate <= 2) {
    return { grade: 'good', label: '良好', color: 'text-info' }
  } else if (lossRate <= 5) {
    return { grade: 'fair', label: '一般', color: 'text-warning' }
  } else {
    return { grade: 'poor', label: '较差', color: 'text-error' }
  }
}
