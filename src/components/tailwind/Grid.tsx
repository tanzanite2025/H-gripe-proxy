import { forwardRef, type HTMLAttributes, type ReactNode } from 'react'

export interface GridProps extends HTMLAttributes<HTMLDivElement> {
  container?: boolean
  item?: boolean
  spacing?: number
  size?: number | { xs?: number; sm?: number; md?: number; lg?: number; xl?: number }
  xs?: number
  sm?: number
  md?: number
  lg?: number
  xl?: number
  children?: ReactNode
}

export const Grid = forwardRef<HTMLDivElement, GridProps>(
  (
    {
      container = false,
      item = false,
      spacing = 0,
      size,
      xs,
      sm,
      md,
      lg,
      xl,
      className = '',
      children,
      ...props
    },
    ref,
  ) => {
    // Handle size prop (MUI Grid2 API)
    if (size !== undefined) {
      if (typeof size === 'number') {
        xs = xs ?? size
      } else {
        xs = xs ?? size.xs
        sm = sm ?? size.sm
        md = md ?? size.md
        lg = lg ?? size.lg
        xl = xl ?? size.xl
      }
    }

    if (container) {
      const gapClass = spacing > 0 ? `gap-${spacing}` : ''
      return (
        <div
          ref={ref}
          className={`grid grid-cols-12 ${gapClass} ${className}`}
          {...props}
        >
          {children}
        </div>
      )
    }

    if (item || xs || sm || md || lg || xl) {
      // 计算列跨度
      const getColSpan = (size?: number) => {
        if (!size) return ''
        return `col-span-${size}`
      }

      const colClasses = [
        xs && getColSpan(xs),
        sm && `sm:${getColSpan(sm)}`,
        md && `md:${getColSpan(md)}`,
        lg && `lg:${getColSpan(lg)}`,
        xl && `xl:${getColSpan(xl)}`,
      ]
        .filter(Boolean)
        .join(' ')

      return (
        <div ref={ref} className={`${colClasses} ${className}`} {...props}>
          {children}
        </div>
      )
    }

    return (
      <div ref={ref} className={className} {...props}>
        {children}
      </div>
    )
  },
)

Grid.displayName = 'Grid'
