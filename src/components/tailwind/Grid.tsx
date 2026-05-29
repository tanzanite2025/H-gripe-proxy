import { forwardRef, type ElementType, type HTMLAttributes, type ReactNode } from 'react'

type ResponsiveGridValue = {
  xs?: number
  sm?: number
  md?: number
  lg?: number
  xl?: number
}

const getResponsiveClass = (
  value: number | ResponsiveGridValue | undefined,
  getClassName: (size: number) => string,
) => {
  if (value === undefined) {
    return ''
  }

  if (typeof value === 'number') {
    return getClassName(value)
  }

  const xsClass = value.xs !== undefined ? getClassName(value.xs) : ''
  const smClass = value.sm !== undefined ? `sm:${getClassName(value.sm)}` : ''
  const mdClass = value.md !== undefined ? `md:${getClassName(value.md)}` : ''
  const lgClass = value.lg !== undefined ? `lg:${getClassName(value.lg)}` : ''
  const xlClass = value.xl !== undefined ? `xl:${getClassName(value.xl)}` : ''

  return [xsClass, smClass, mdClass, lgClass, xlClass].filter(Boolean).join(' ')
}

export interface GridProps extends HTMLAttributes<HTMLDivElement> {
  container?: boolean
  item?: boolean
  spacing?: number | ResponsiveGridValue
  columns?: number | ResponsiveGridValue
  size?: number | ResponsiveGridValue
  xs?: number
  sm?: number
  md?: number
  lg?: number
  xl?: number
  component?: ElementType
  children?: ReactNode
}

export const Grid = forwardRef<HTMLDivElement, GridProps>(
  (
    {
      container = false,
      item = false,
      spacing = 0,
      columns,
      size,
      component,
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
    const Comp = (component ?? 'div') as ElementType

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
      const gapClass = getResponsiveClass(spacing, (current) => `gap-${current}`)
      const colsClass =
        getResponsiveClass(columns ?? 12, (current) => `grid-cols-${current}`) ||
        'grid-cols-12'

      return (
        <Comp
          ref={ref}
          className={`grid ${colsClass} ${gapClass} ${className}`}
          {...props}
        >
          {children}
        </Comp>
      )
    }

    if (item || xs || sm || md || lg || xl) {
      const colClasses = getResponsiveClass(
        { xs, sm, md, lg, xl },
        (current) => `col-span-${current}`,
      )

      return (
        <Comp ref={ref} className={`${colClasses} ${className}`} {...props}>
          {children}
        </Comp>
      )
    }

    return (
      <Comp ref={ref} className={className} {...props}>
        {children}
      </Comp>
    )
  },
)

Grid.displayName = 'Grid'
