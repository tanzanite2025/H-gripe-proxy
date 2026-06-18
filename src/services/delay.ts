import { invoke } from '@tauri-apps/api/core'

import type { ProxyDelay } from '@/types/mihomo'
import { debugLog } from '@/utils/misc'

import { getDelayTestConfig } from './adaptive-config'
import { planLatencyTest } from './cmds/runtime'
import {
  DEFAULT_DELAY_TIMEOUT,
  normalizeDelayTestUrl,
} from './delay-config'
import { networkMonitor } from './network-monitor'

export async function delayRuntimeProxy(
  proxyName: string,
  testUrl: string,
  timeout: number,
) {
  return invoke<ProxyDelay>('delay_runtime_proxy', {
    proxyName,
    testUrl,
    timeout,
  })
}

export async function delayRuntimeGroup(
  groupName: string,
  testUrl: string,
  timeout: number,
  keepFixed = false,
) {
  return invoke<Record<string, number>>('delay_runtime_group', {
    groupName,
    testUrl,
    timeout,
    keepFixed,
  })
}

const hashKey = (name: string, group: string) => `${group ?? ''}::${name}`

export interface DelayUpdate {
  delay: number
  elapsed?: number
  updatedAt: number
}

const CACHE_TTL = 30 * 60 * 1000

class DelayManager {
  private cache = new Map<string, DelayUpdate>()
  private urlMap = new Map<string, string>()

  // 每个节点的监听
  private listenerMap = new Map<string, (update: DelayUpdate) => void>()

  // 每个分组的监听
  private groupListenerMap = new Map<string, () => void>()

  private pendingItemUpdates = new Map<string, DelayUpdate[]>()
  private pendingGroupUpdates = new Set<string>()
  private itemFlushScheduled = false
  private groupFlushScheduled = false

  // 取消控制器
  private abortControllers = new Map<string, AbortController>()

  private scheduleOnNextFrame(run: () => void): void {
    if (typeof window !== 'undefined') {
      if (typeof window.requestAnimationFrame === 'function') {
        window.requestAnimationFrame(run)
        return
      }
      if (typeof window.setTimeout === 'function') {
        window.setTimeout(run, 0)
        return
      }
    }

    Promise.resolve().then(run)
  }

  private scheduleItemFlush() {
    if (this.itemFlushScheduled) return
    this.itemFlushScheduled = true

    this.scheduleOnNextFrame(() => {
      this.itemFlushScheduled = false
      const updates = this.pendingItemUpdates
      this.pendingItemUpdates = new Map()

      updates.forEach((queue, key) => {
        const listener = this.listenerMap.get(key)
        if (!listener) return

        queue.forEach((update) => {
          try {
            listener(update)
          } catch (error) {
            console.error(
              `[DelayManager] 通知节点延迟监听器失败: ${key}`,
              error,
            )
          }
        })
      })
    })
  }

  private scheduleGroupFlush() {
    if (this.groupFlushScheduled) return
    this.groupFlushScheduled = true

    this.scheduleOnNextFrame(() => {
      this.groupFlushScheduled = false
      const groups = this.pendingGroupUpdates
      this.pendingGroupUpdates = new Set()

      groups.forEach((group) => {
        const listener = this.groupListenerMap.get(group)
        if (!listener) return
        try {
          listener()
        } catch (error) {
          console.error(
            `[DelayManager] 通知分组延迟监听器失败: ${group}`,
            error,
          )
        }
      })
    })
  }

  private queueGroupNotification(group: string) {
    this.pendingGroupUpdates.add(group)
    this.scheduleGroupFlush()
  }

  setUrl(group: string, url: string) {
    debugLog(`[DelayManager] 设置测试URL，组: ${group}, URL: ${url}`)
    this.urlMap.set(group, normalizeDelayTestUrl(url))
  }

  getUrl(group: string) {
    const url = normalizeDelayTestUrl(this.urlMap.get(group))
    debugLog(
      `[DelayManager] 获取测试URL，组: ${group}, URL: ${url || '未设置'}`,
    )
    // 如果未设置URL，返回默认URL
    return url
  }

  setListener(
    name: string,
    group: string,
    listener: (update: DelayUpdate) => void,
  ) {
    const key = hashKey(name, group)
    this.listenerMap.set(key, listener)
  }

  removeListener(name: string, group: string) {
    const key = hashKey(name, group)
    this.listenerMap.delete(key)
  }

  setGroupListener(group: string, listener: () => void) {
    this.groupListenerMap.set(group, listener)
  }

  removeGroupListener(group: string) {
    this.groupListenerMap.delete(group)
  }

  setDelay(
    name: string,
    group: string,
    delay: number,
    meta?: { elapsed?: number },
  ): DelayUpdate {
    const key = hashKey(name, group)
    debugLog(
      `[DelayManager] 设置延迟，代理: ${name}, 组: ${group}, 延迟: ${delay}`,
    )
    const update: DelayUpdate = {
      delay,
      elapsed: meta?.elapsed,
      updatedAt: Date.now(),
    }

    this.cache.set(key, update)

    const queue = this.pendingItemUpdates.get(key)
    if (queue) {
      queue.push(update)
    } else {
      this.pendingItemUpdates.set(key, [update])
    }
    this.scheduleItemFlush()

    return update
  }

  getDelayUpdate(name: string, group: string) {
    const key = hashKey(name, group)
    const entry = this.cache.get(key)
    if (!entry) return undefined

    if (Date.now() - entry.updatedAt > CACHE_TTL) {
      this.cache.delete(key)
      return undefined
    }

    return { ...entry }
  }

  getDelay(name: string, group: string) {
    const update = this.getDelayUpdate(name, group)
    return update ? update.delay : -1
  }

  /// 暂时修复provider的节点延迟排序的问题
  getDelayFix(proxy: IProxyItem, group: string) {
    if (!proxy.provider) {
      const update = this.getDelayUpdate(proxy.name, group)
      if (update && (update.delay >= 0 || update.delay === -2)) {
        return update.delay
      }
    }

    // 添加 history 属性的安全检查
    if (proxy.history && proxy.history.length > 0) {
      // 0ms以error显示
      return proxy.history[proxy.history.length - 1].delay ?? 1e6
    }
    return -1
  }

  async checkDelay(
    name: string,
    group: string,
    timeout?: number,
    signal?: AbortSignal,
  ): Promise<DelayUpdate> {
    // 使用自适应配置
    const config = getDelayTestConfig()
    const effectiveTimeout = timeout ?? config.timeout

    // 检查网络状态
    if (!networkMonitor.isOnline()) {
      debugLog(`[DelayManager] 网络离线，跳过延迟测试: ${name}`)
      return this.setDelay(name, group, 1e6) // 设置为错误状态
    }

    debugLog(
      `[DelayManager] 开始测试延迟，代理: ${name}, 组: ${group}, 超时: ${effectiveTimeout}ms`,
    )

    // 先将状态设置为测试中
    this.setDelay(name, group, -2)

    const startTime = Date.now()

    try {
      // 检查是否已取消
      if (signal?.aborted) {
        throw new Error('测试已取消')
      }

      const url = this.getUrl(group)
      debugLog(`[DelayManager] 调用API测试延迟，代理: ${name}, URL: ${url}`)

      // 设置超时处理, delay = 0 为超时
      const timeoutPromise = new Promise<ProxyDelay>((resolve) => {
        setTimeout(() => resolve({ delay: 0 }), effectiveTimeout)
      })

      // 使用Promise.race来实现超时控制
      const result = await Promise.race([
        delayRuntimeProxy(name, url, effectiveTimeout),
        timeoutPromise,
      ])

      // 确保至少显示最小加载时间
      const elapsedTime = Date.now() - startTime
      const minLoadingTime = config.minLoadingTime
      if (elapsedTime < minLoadingTime) {
        await new Promise((resolve) =>
          setTimeout(resolve, minLoadingTime - elapsedTime),
        )
      }

      const delay = result.delay
      const elapsed = elapsedTime
      debugLog(`[DelayManager] 延迟测试完成，代理: ${name}, 结果: ${delay}ms`)

      return this.setDelay(name, group, delay, { elapsed })
    } catch (error) {
      // 检查是否是取消错误
      if (signal?.aborted || (error as Error).message === '测试已取消') {
        debugLog(`[DelayManager] 延迟测试已取消，代理: ${name}`)
        return this.setDelay(name, group, -1) // 恢复为未测试状态
      }

      // 确保至少显示最小加载时间
      const config = getDelayTestConfig()
      await new Promise((resolve) =>
        setTimeout(resolve, config.minLoadingTime),
      )
      console.error(`[DelayManager] 延迟测试出错，代理: ${name}`, error)
      const delay = 1e6 // error
      const elapsed = Date.now() - startTime

      return this.setDelay(name, group, delay, { elapsed })
    }
  }

  async checkListDelay(
    nameList: string[],
    group: string,
    timeout?: number,
    concurrency?: number,
  ) {
    // 检查网络状态
    if (!networkMonitor.isOnline()) {
      debugLog(`[DelayManager] 网络离线，跳过批量延迟测试: ${group}`)
      return
    }

    const plan = await planLatencyTest({
      proxyNames: nameList,
      group,
      url: this.getUrl(group),
      timeoutMs: timeout,
      concurrency,
      networkQuality: networkMonitor.getQuality(),
    })

    if (plan.status === 'skipped') {
      debugLog(`[DelayManager] 批量测试跳过，组: ${group}, 原因: ${plan.reason}`)
      return
    }

    const effectiveTimeout = plan.timeoutMs
    const actualConcurrency = plan.concurrency
    this.setUrl(group, plan.normalizedUrl)

    debugLog(
      `[DelayManager] 批量测试延迟开始，组: ${group}, 数量: ${plan.scheduledCount}, 并发数: ${actualConcurrency}`,
    )

    const names = plan.proxyNames
    // 设置正在延迟测试中
    names.forEach((name) => this.setDelay(name, group, -2))

    // 创建 AbortController
    const controller = new AbortController()
    this.abortControllers.set(group, controller)

    let index = 0
    const startTime = Date.now()
    const listener = this.groupListenerMap.get(group)

    const help = async (): Promise<void> => {
      const currName = names[index++]
      if (!currName) return

      try {
        // 检查是否已取消
        if (controller.signal.aborted) {
          debugLog(`[DelayManager] 批量测试已取消: ${group}`)
          return
        }

        // 确保API调用前状态为测试中
        this.setDelay(currName, group, -2)

        // 添加一些随机延迟，避免所有请求同时发出和返回
        if (index > 1) {
          // 第一个不延迟，保持响应性
          await new Promise((resolve) =>
            setTimeout(resolve, Math.random() * 200),
          )
        }

        await this.checkDelay(
          currName,
          group,
          effectiveTimeout,
          controller.signal,
        )
        if (listener) {
          this.queueGroupNotification(group)
        }
      } catch (error) {
        // 如果是取消错误，直接返回
        if (controller.signal.aborted) {
          return
        }

        console.error(
          `[DelayManager] 批量测试单个代理出错，代理: ${currName}`,
          error,
        )
        // 设置为错误状态
        this.setDelay(currName, group, 1e6)
      }

      return help()
    }

    debugLog(`[DelayManager] 实际并发数: ${actualConcurrency}`)

    const promiseList: Promise<void>[] = []
    for (let i = 0; i < actualConcurrency; i++) {
      promiseList.push(help())
    }

    try {
      await Promise.all(promiseList)
      const totalTime = Date.now() - startTime
      debugLog(
        `[DelayManager] 批量测试延迟完成，组: ${group}, 总耗时: ${totalTime}ms`,
      )
    } finally {
      // 清理 AbortController
      this.abortControllers.delete(group)
    }
  }

  /**
   * 取消组的延迟测试
   */
  cancelGroupTest(group: string): void {
    const controller = this.abortControllers.get(group)
    if (controller) {
      controller.abort()
      debugLog(`[DelayManager] 取消组延迟测试: ${group}`)
    }
  }

  /**
   * 检查组是否正在测试
   */
  isGroupTesting(group: string): boolean {
    return this.abortControllers.has(group)
  }

  formatDelay(delay: number, timeout = DEFAULT_DELAY_TIMEOUT) {
    if (delay === -1) return '-'
    if (delay === -2) return 'testing'
    if (delay === 0 || (delay >= timeout && delay <= 1e5)) return 'Timeout'
    if (delay > 1e5) return 'Error'
    return `${delay}`
  }

  formatDelayColor(delay: number, timeout = DEFAULT_DELAY_TIMEOUT) {
    if (delay < 0) return ''
    if (delay === 0 || delay >= timeout) return 'error.main'
    if (delay >= DEFAULT_DELAY_TIMEOUT) return 'error.main'
    if (delay >= 400) return 'warning.main'
    if (delay >= 250) return 'primary.main'
    return 'success.main'
  }
}

export default new DelayManager()
