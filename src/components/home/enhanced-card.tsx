import React, { forwardRef, ReactNode } from 'react'
import { cn } from '@/utils/cn'

// 自定义卡片组件接口
interface EnhancedCardProps {
  title: ReactNode
  icon: ReactNode
  action?: ReactNode
  children: ReactNode
  iconColor?: 'primary' | 'secondary' | 'error' | 'warning' | 'info' | 'success'
  minHeight?: number | string
  noContentPadding?: boolean
}

const iconColorMap = {
  primary: 'bg-primary/10 text-primary dark:bg-primary-dark-mode/10 dark:text-primary-dark-mode',
  secondary: 'bg-gray-500/10 text-gray-500 dark:bg-gray-400/10 dark:text-gray-400',
  error: 'bg-red-500/10 text-red-500 dark:bg-red-400/10 dark:text-red-400',
  warning: 'bg-yellow-500/10 text-yellow-500 dark:bg-yellow-400/10 dark:text-yellow-400',
  info: 'bg-blue-500/10 text-blue-500 dark:bg-blue-400/10 dark:text-blue-400',
  success: 'bg-green-500/10 text-green-500 dark:bg-green-400/10 dark:text-green-400',
}

// 自定义卡片组件
export const EnhancedCard = forwardRef<HTMLElement, EnhancedCardProps>(
  (
    {
      title,
      icon,
      action,
      children,
      iconColor = 'primary',
      minHeight,
      noContentPadding = false,
    },
    ref,
  ) => {
    return (
      <section
        className="uds-card-container uds-surface flex h-full flex-col"
        ref={ref}
      >
        <div className="uds-card-header flex items-center justify-between px-4 py-2">
          <div className="flex min-w-0 flex-1 items-center overflow-hidden">
            {icon && (
              <div
                className={cn(
                  'mr-3 flex h-[38px] w-[38px] flex-shrink-0 items-center justify-center rounded-xl',
                  iconColorMap[iconColor]
                )}
              >
                {icon}
              </div>
            )}
            <div className="min-w-0 flex-1">
              {typeof title === 'string' ? (
                <h3
                  className="uds-card-title block max-w-full overflow-hidden text-ellipsis whitespace-nowrap text-lg font-semibold"
                  title={title}
                >
                  {title}
                </h3>
              ) : (
                <div className="block max-w-full overflow-hidden text-ellipsis whitespace-nowrap">
                  {title}
                </div>
              )}
            </div>
          </div>
          {action && <div className="ml-4 flex-shrink-0">{action}</div>}
        </div>
        <div
          className={cn(
            'uds-card-content flex flex-1 flex-col',
            noContentPadding ? 'p-0' : 'p-4'
          )}
          style={minHeight ? { minHeight } : undefined}
        >
          {children}
        </div>
      </section>
    )
  },
)
