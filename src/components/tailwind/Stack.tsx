import { forwardRef, type HTMLAttributes, type ReactNode } from 'react'

export interface StackProps extends HTMLAttributes<HTMLDivElement> {
  direction?: 'row' | 'column'
  spacing?: number
  align?: 'start' | 'center' | 'end' | 'stretch'
  justify?: 'start' | 'center' | 'end' | 'between' | 'around' | 'evenly'
  useFlexGap?: boolean
  children: ReactNode
}

export const Stack = forwardRef<HTMLDivElement, StackProps>(
  (
    {
      direction = 'column',
      spacing = 2,
      align = 'stretch',
      justify = 'start',
      useFlexGap = true, // ignored in Tailwind, gap is always flex gap
      className = '',
      children,
      ...props
    },
    ref,
  ) => {
    const directionClass = direction === 'row' ? 'flex-row' : 'flex-col'

    const spacingClass = direction === 'row' ? `gap-${spacing}` : `gap-${spacing}`

    const alignClasses = {
      start: 'items-start',
      center: 'items-center',
      end: 'items-end',
      stretch: 'items-stretch',
    }

    const justifyClasses = {
      start: 'justify-start',
      center: 'justify-center',
      end: 'justify-end',
      between: 'justify-between',
      around: 'justify-around',
      evenly: 'justify-evenly',
    }

    return (
      <div
        ref={ref}
        className={`flex ${directionClass} ${spacingClass} ${alignClasses[align]} ${justifyClasses[justify]} ${className}`}
        {...props}
      >
        {children}
      </div>
    )
  },
)

Stack.displayName = 'Stack'
