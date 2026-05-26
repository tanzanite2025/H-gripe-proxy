/**
 * 节流函数 - 用于优化滚动性能
 */
export function throttle<T extends (...args: any[]) => any>(
  func: T,
  wait: number,
): (...args: Parameters<T>) => void {
  let timer: ReturnType<typeof setTimeout> | null = null
  let previous = 0
  let lastArgs: Parameters<T> | null = null

  const run = (args: Parameters<T>) => {
    previous = Date.now()
    timer = null
    lastArgs = null
    func(...args)
  }

  return function (...args: Parameters<T>) {
    const now = Date.now()
    const remaining = wait - (now - previous)
    lastArgs = args

    if (remaining <= 0 || remaining > wait) {
      if (timer) {
        clearTimeout(timer)
      }
      run(args)
    } else if (!timer) {
      timer = setTimeout(() => {
        if (lastArgs) {
          run(lastArgs)
        }
      }, remaining)
    }
  }
}


