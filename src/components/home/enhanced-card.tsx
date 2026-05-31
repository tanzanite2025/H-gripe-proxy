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
  fixedHeight?: number | string
  noContentPadding?: boolean
}

const iconColorMap = {
  primary: 'bg-primary bg-opacity-10 text-primary dark:bg-primary-dark-mode dark:bg-opacity-10 dark:text-primary-dark-mode',
  secondary: 'bg-gray-500 bg-opacity-10 text-gray-500 dark:bg-gray-400 dark:bg-opacity-10 dark:text-gray-400',
  error: 'bg-red-500 bg-opacity-10 text-red-500 dark:bg-red-400 dark:bg-opacity-10 dark:text-red-400',
  warning: 'bg-yellow-500 bg-opacity-10 text-yellow-500 dark:bg-yellow-400 dark:bg-opacity-10 dark:text-yellow-400',
  info: 'bg-teal-500 bg-opacity-10 text-teal-500 dark:bg-teal-400 dark:bg-opacity-10 dark:text-teal-400',
  success: 'bg-green-500 bg-opacity-10 text-green-500 dark:bg-green-400 dark:bg-opacity-10 dark:text-green-400',
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
      fixedHeight,
      noContentPadding = false,
    },
    ref,
  ) => {
    return (
      <section
        className={cn('uds-card-container uds-surface flex flex-col', fixedHeight && 'uds-card-fixed-height')}
        ref={ref}
        style={fixedHeight ? { '--card-fixed-h': typeof fixedHeight === 'number' ? `${fixedHeight}px` : fixedHeight } as React.CSSProperties : undefined}
      >
        <div className="uds-card-header flex items-center justify-between px-3 py-2">
          <div className="flex min-w-0 flex-1 items-center overflow-hidden">
            {icon && (
              <div
                className={cn(
                  'mr-2 flex h-[28px] w-[28px] flex-shrink-0 items-center justify-center rounded-lg',
                  iconColorMap[iconColor]
                )}
              >
                {icon}
              </div>
            )}
            <div className="min-w-0 flex-1">
              {typeof title === 'string' ? (
                <h3
                  className="uds-card-title block max-w-full overflow-hidden text-ellipsis whitespace-nowrap"
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
            noContentPadding ? 'p-0' : 'p-3',
          )}
          style={minHeight ? { minHeight } : undefined}
        >
          {children}
        </div>
      </section>
    )
  },
)
