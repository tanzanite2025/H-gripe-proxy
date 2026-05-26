/**
 * 自适应配置服务
 * 根据网络质量动态调整超时、重试等参数
 */

import { networkMonitor, type NetworkQuality } from './network-monitor'

export interface AdaptiveConfig {
  timeout: number
  retries: number
  retryDelay: number
  concurrency: number
}

/**
 * 根据网络质量获取自适应配置
 */
export const getAdaptiveConfig = (
  quality?: NetworkQuality,
): AdaptiveConfig => {
  const networkQuality = quality || networkMonitor.getQuality()

  switch (networkQuality) {
    case 'good':
      return {
        timeout: 5000, // 5秒超时
        retries: 2, // 重试2次
        retryDelay: 1000, // 1秒重试间隔
        concurrency: 10, // 并发10个
      }
    case 'poor':
      return {
        timeout: 10000, // 10秒超时（弱网时延长）
        retries: 3, // 重试3次（增加重试）
        retryDelay: 3000, // 3秒重试间隔（延长间隔）
        concurrency: 3, // 并发3个（降低并发）
      }
    case 'offline':
      return {
        timeout: 0, // 不请求
        retries: 0, // 不重试
        retryDelay: 0,
        concurrency: 0,
      }
  }
}

/**
 * 获取延迟测试的自适应配置
 */
export const getDelayTestConfig = (quality?: NetworkQuality) => {
  const networkQuality = quality || networkMonitor.getQuality()

  switch (networkQuality) {
    case 'good':
      return {
        timeout: 5000,
        concurrency: 10,
        minLoadingTime: 500, // 最小加载时间
      }
    case 'poor':
      return {
        timeout: 10000,
        concurrency: 3,
        minLoadingTime: 300, // 弱网时减少最小加载时间
      }
    case 'offline':
      return {
        timeout: 0,
        concurrency: 0,
        minLoadingTime: 0,
      }
  }
}

/**
 * 获取 IP 检测的自适应配置
 */
export const getIpCheckConfig = (quality?: NetworkQuality) => {
  const networkQuality = quality || networkMonitor.getQuality()

  switch (networkQuality) {
    case 'good':
      return {
        timeout: 5000,
        retries: 2,
        minTimeout: 1000,
        maxTimeout: 4000,
      }
    case 'poor':
      return {
        timeout: 10000,
        retries: 3,
        minTimeout: 2000,
        maxTimeout: 8000,
      }
    case 'offline':
      return {
        timeout: 0,
        retries: 0,
        minTimeout: 0,
        maxTimeout: 0,
      }
  }
}
