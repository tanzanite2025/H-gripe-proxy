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
      outlined: 'border border-divider',
      elevation: 'shadow-md',
    }

    return (
      <div
        ref={ref}
        className={`bg-card rounded-lg ${variantClasses[variant]} ${className}`}
        style={style}
        {...props}
      >
        {children}
      </div>
    )
  }
)

Card.displayName = 'Card'
