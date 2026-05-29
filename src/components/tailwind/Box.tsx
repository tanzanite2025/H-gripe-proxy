import { forwardRef, type ElementType, type HTMLAttributes, type ReactNode } from 'react'

export interface BoxProps extends HTMLAttributes<HTMLDivElement> {
  as?: ElementType
  component?: ElementType
  children?: ReactNode
}

export const Box = forwardRef<HTMLDivElement, BoxProps>(
  ({ as: asComponent, component, className = '', children, ...props }, ref) => {
    const Comp = (component ?? asComponent ?? 'div') as ElementType
    return (
      <Comp ref={ref} className={className} {...props}>
        {children}
      </Comp>
    )
  },
)

Box.displayName = 'Box'
