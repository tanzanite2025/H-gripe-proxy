import React from 'react'

export interface ButtonGroupProps {
  children?: React.ReactNode
  className?: string
  variant?: 'text' | 'outlined' | 'contained' | 'primary'
  size?: 'small' | 'medium' | 'large'
  style?: React.CSSProperties
}

export const ButtonGroup = React.forwardRef<HTMLDivElement, ButtonGroupProps>(
  ({ children, className = '', variant: _variant, size: _size, style, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={`inline-flex ${className}`}
        role="group"
        style={style}
        {...props}
      >
        {children}
      </div>
    )
  }
)

ButtonGroup.displayName = 'ButtonGroup'
