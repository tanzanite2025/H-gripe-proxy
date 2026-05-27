/**
 * 延迟测试进度组件
 * 显示批量延迟测试的进度和取消按钮
 */

import { Button } from '@/components/tailwind'
import { useEffect, useState } from 'react'

interface DelayTestProgressProps {
  total: number
  completed: number
  testing: boolean
  onCancel?: () => void
}

export const DelayTestProgress = ({
  total,
  completed,
  testing,
  onCancel,
}: DelayTestProgressProps) => {
  const [progress, setProgress] = useState(0)

  useEffect(() => {
    if (total > 0) {
      setProgress((completed / total) * 100)
    }
  }, [completed, total])

  if (!testing) {
    return null
  }

  return (
    <div className="mb-2 w-full">
      <div className="mb-1 flex items-center justify-between">
        <span className="text-xs text-gray-600 dark:text-gray-400">
          正在测试延迟... {completed}/{total}
        </span>
        {onCancel && (
          <Button size="small" variant="danger" onClick={onCancel}>
            取消
          </Button>
        )}
      </div>
      <div className="h-1 w-full overflow-hidden rounded-full bg-gray-200 dark:bg-gray-700">
        <div
          className="h-full bg-primary transition-all duration-300 dark:bg-primary-dark-mode"
          style={{ width: `${progress}%` }}
        />
      </div>
    </div>
  )
}

/**
 * 简单的延迟测试状态指示器
 */
export const DelayTestIndicator = ({ testing }: { testing: boolean }) => {
  if (!testing) {
    return null
  }

  return (
    <div className="flex items-center gap-1">
      <style>{`
        @keyframes pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.5; }
        }
        .pulse-dot {
          animation: pulse 1.5s ease-in-out infinite;
        }
      `}</style>
      <div className="pulse-dot h-2 w-2 rounded-full bg-primary dark:bg-primary-dark-mode" />
      <span className="text-xs text-gray-600 dark:text-gray-400">测试中...</span>
    </div>
  )
}
