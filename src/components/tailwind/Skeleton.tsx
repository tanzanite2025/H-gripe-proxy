import { type HTMLAttributes } from 'react'

export interface SkeletonProps extends HTMLAttributes<HTMLDivElement> {
  variant?: 'text' | 'circular' | 'rectangular'
  width?: string | number
  height?: string | number
  animation?: 'pulse' | 'wave' | 'none'
}

export const Skeleton = ({
  variant = 'text',
  width,
  height,
  animation = 'pulse',
  className = '',
  style,
  ...props
}: SkeletonProps) => {
  const baseClasses = 'bg-card text-text-primary hover:bg-white/10'

  const variantClasses = {
    text: 'rounded',
    circular: 'rounded-full',
    rectangular: 'rounded-lg',
  }

  const animationClasses = {
    pulse: 'animate-pulse',
    wave: 'animate-pulse', // 可以自定义 wave 动画
    none: '',
  }

  const defaultHeight = variant === 'text' ? '1em' : height || '100%'

  return (
    <div
      className={`${baseClasses} ${variantClasses[variant]} ${animationClasses[animation]} ${className}`}
      style={{
        width: width || '100%',
        height: defaultHeight,
        ...style,
      }}
      aria-busy="true"
      aria-live="polite"
      {...props}
    />
  )
}

Skeleton.displayName = 'Skeleton'
