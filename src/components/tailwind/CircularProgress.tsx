import React from 'react'

export interface CircularProgressProps {
  size?: number
  color?: 'inherit' | 'primary' | 'secondary'
  className?: string
}

export const CircularProgress = React.forwardRef<HTMLDivElement, CircularProgressProps>(
  ({ size = 40, color = 'primary', className = '', ...props }, ref) => {
    const colorClasses = {
      inherit: 'border-current',
      primary: 'border-blue-500',
      secondary: 'border-purple-500',
    }

    return (
      <div
        ref={ref}
        className={`inline-block animate-spin rounded-full border-2 border-solid border-t-transparent ${colorClasses[color]} ${className}`}
        style={{ width: size, height: size }}
        role="progressbar"
        {...props}
      />
    )
  }
)

CircularProgress.displayName = 'CircularProgress'
