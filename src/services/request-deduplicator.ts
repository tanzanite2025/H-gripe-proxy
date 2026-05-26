/**
 * 请求去重服务
 * 避免同时发起多个相同的请求
 */

class RequestDeduplicator {
  private pending = new Map<string, Promise<any>>()

  /**
   * 去重执行请求
   * @param key 请求的唯一标识
   * @param fn 请求函数
   * @returns 请求结果
   */
  async dedupe<T>(key: string, fn: () => Promise<T>): Promise<T> {
    // 如果已有相同请求在进行中，直接返回
    if (this.pending.has(key)) {
      console.debug(`[RequestDeduplicator] 请求去重: ${key}`)
      return this.pending.get(key)!
    }

    // 创建新请求
    const promise = fn().finally(() => {
      this.pending.delete(key)
    })

    this.pending.set(key, promise)
    return promise
  }

  /**
   * 检查是否有进行中的请求
   */
  hasPending(key: string): boolean {
    return this.pending.has(key)
  }

  /**
   * 获取所有进行中的请求键
   */
  getPendingKeys(): string[] {
    return Array.from(this.pending.keys())
  }

  /**
   * 清除所有进行中的请求
   */
  clear(): void {
    this.pending.clear()
  }
}

// 导出单例
export const deduplicator = new RequestDeduplicator()
