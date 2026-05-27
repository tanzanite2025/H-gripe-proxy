import React from 'react'

export interface TypographyProps {
  children?: React.ReactNode
  variant?: 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6' | 'subtitle1' | 'subtitle2' | 'body1' | 'body2' | 'caption' | 'overline'
  className?: string
  component?: keyof JSX.IntrinsicElements
}

export const Typography = React.forwardRef<HTMLElement, TypographyProps>(
  ({ children, variant = 'body1', className = '', component, ...props }, ref) => {
    const variantClasses = {
      h1: 'text-6xl font-light',
      h2: 'text-5xl font-light',
      h3: 'text-4xl font-normal',
      h4: 'text-3xl font-normal',
      h5: 'text-2xl font-normal',
      h6: 'text-xl font-medium',
      subtitle1: 'text-base font-normal',
      subtitle2: 'text-sm font-medium',
      body1: 'text-base font-normal',
      body2: 'text-sm font-normal',
      caption: 'text-xs font-normal',
      overline: 'text-xs font-normal uppercase tracking-wider',
    }

    const defaultComponents = {
      h1: 'h1',
      h2: 'h2',
      h3: 'h3',
      h4: 'h4',
      h5: 'h5',
      h6: 'h6',
      subtitle1: 'h6',
      subtitle2: 'h6',
      body1: 'p',
      body2: 'p',
      caption: 'span',
      overline: 'span',
    }

    const Component = (component || defaultComponents[variant]) as any

    return (
      <Component
        ref={ref}
        className={`${variantClasses[variant]} ${className}`}
        {...props}
      >
        {children}
      </Component>
    )
  }
)

Typography.displayName = 'Typography'
