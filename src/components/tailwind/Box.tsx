import { forwardRef, type HTMLAttributes, type ReactNode } from 'react'

export interface BoxProps extends HTMLAttributes<HTMLDivElement> {
  as?: keyof JSX.IntrinsicElements
  children?: ReactNode
}

export const Box = forwardRef<HTMLDivElement, BoxProps>(
  ({ as: Component = 'div', className = '', children, ...props }, ref) => {
    return (
      <Component ref={ref} className={className} {...props}>
        {children}
      </Component>
    )
  },
)

Box.displayName = 'Box'
