import React from 'react'

export interface CardProps {
  children?: React.ReactNode
  className?: string
  variant?: 'outlined' | 'elevation'
  style?: React.CSSProperties
}

export const Card = React.forwardRef<HTMLDivElement, CardProps>(
  ({ children, className = '', variant = 'elevation', style, ...props }, ref) => {
    const variantClasses = {
      outlined: 'border border-gray-200 dark:border-gray-700',
      elevation: 'shadow-md',
    }

    return (
      <div
        ref={ref}
        className={`bg-white dark:bg-gray-800 rounded-lg ${variantClasses[variant]} ${className}`}
        style={style}
        {...props}
      >
        {children}
      </div>
    )
  }
)

Card.displayName = 'Card'
