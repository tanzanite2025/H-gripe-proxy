import { forwardRef, type ElementType, type HTMLAttributes, type ReactNode } from 'react'

type ResponsiveGridValue = {
  xs?: number
  sm?: number
  md?: number
  lg?: number
  xl?: number
}

const COL_SPAN_MAP: Record<string, Record<number, string>> = {
  xs: {
    1: 'col-span-1', 2: 'col-span-2', 3: 'col-span-3', 4: 'col-span-4',
    5: 'col-span-5', 6: 'col-span-6', 7: 'col-span-7', 8: 'col-span-8',
    9: 'col-span-9', 10: 'col-span-10', 11: 'col-span-11', 12: 'col-span-12'
  },
  sm: {
    1: 'sm:col-span-1', 2: 'sm:col-span-2', 3: 'sm:col-span-3', 4: 'sm:col-span-4',
    5: 'sm:col-span-5', 6: 'sm:col-span-6', 7: 'sm:col-span-7', 8: 'sm:col-span-8',
    9: 'sm:col-span-9', 10: 'sm:col-span-10', 11: 'sm:col-span-11', 12: 'sm:col-span-12'
  },
  md: {
    1: 'md:col-span-1', 2: 'md:col-span-2', 3: 'md:col-span-3', 4: 'md:col-span-4',
    5: 'md:col-span-5', 6: 'md:col-span-6', 7: 'md:col-span-7', 8: 'md:col-span-8',
    9: 'md:col-span-9', 10: 'md:col-span-10', 11: 'md:col-span-11', 12: 'md:col-span-12'
  },
  lg: {
    1: 'lg:col-span-1', 2: 'lg:col-span-2', 3: 'lg:col-span-3', 4: 'lg:col-span-4',
    5: 'lg:col-span-5', 6: 'lg:col-span-6', 7: 'lg:col-span-7', 8: 'lg:col-span-8',
    9: 'lg:col-span-9', 10: 'lg:col-span-10', 11: 'lg:col-span-11', 12: 'lg:col-span-12'
  },
  xl: {
    1: 'xl:col-span-1', 2: 'xl:col-span-2', 3: 'xl:col-span-3', 4: 'xl:col-span-4',
    5: 'xl:col-span-5', 6: 'xl:col-span-6', 7: 'xl:col-span-7', 8: 'xl:col-span-8',
    9: 'xl:col-span-9', 10: 'xl:col-span-10', 11: 'xl:col-span-11', 12: 'xl:col-span-12'
  }
}

const GRID_COLS_MAP: Record<string, Record<number, string>> = {
  xs: {
    1: 'grid-cols-1', 2: 'grid-cols-2', 3: 'grid-cols-3', 4: 'grid-cols-4',
    5: 'grid-cols-5', 6: 'grid-cols-6', 7: 'grid-cols-7', 8: 'grid-cols-8',
    9: 'grid-cols-9', 10: 'grid-cols-10', 11: 'grid-cols-11', 12: 'grid-cols-12'
  },
  sm: {
    1: 'sm:grid-cols-1', 2: 'sm:grid-cols-2', 3: 'sm:grid-cols-3', 4: 'sm:grid-cols-4',
    5: 'sm:grid-cols-5', 6: 'sm:grid-cols-6', 7: 'sm:grid-cols-7', 8: 'sm:grid-cols-8',
    9: 'sm:grid-cols-9', 10: 'sm:grid-cols-10', 11: 'sm:grid-cols-11', 12: 'sm:grid-cols-12'
  },
  md: {
    1: 'md:grid-cols-1', 2: 'md:grid-cols-2', 3: 'md:grid-cols-3', 4: 'md:grid-cols-4',
    5: 'md:grid-cols-5', 6: 'md:grid-cols-6', 7: 'md:grid-cols-7', 8: 'md:grid-cols-8',
    9: 'md:grid-cols-9', 10: 'md:grid-cols-10', 11: 'md:grid-cols-11', 12: 'md:grid-cols-12'
  },
  lg: {
    1: 'lg:grid-cols-1', 2: 'lg:grid-cols-2', 3: 'lg:grid-cols-3', 4: 'lg:grid-cols-4',
    5: 'lg:grid-cols-5', 6: 'lg:grid-cols-6', 7: 'lg:grid-cols-7', 8: 'lg:grid-cols-8',
    9: 'lg:grid-cols-9', 10: 'lg:grid-cols-10', 11: 'lg:grid-cols-11', 12: 'lg:grid-cols-12'
  },
  xl: {
    1: 'xl:grid-cols-1', 2: 'xl:grid-cols-2', 3: 'xl:grid-cols-3', 4: 'xl:grid-cols-4',
    5: 'xl:grid-cols-5', 6: 'xl:grid-cols-6', 7: 'xl:grid-cols-7', 8: 'xl:grid-cols-8',
    9: 'xl:grid-cols-9', 10: 'xl:grid-cols-10', 11: 'xl:grid-cols-11', 12: 'xl:grid-cols-12'
  }
}

const GAP_MAP: Record<string, Record<number, string>> = {
  xs: {
    0: 'gap-0', 1: 'gap-1', 2: 'gap-2', 3: 'gap-3', 4: 'gap-4', 5: 'gap-5', 6: 'gap-6',
    8: 'gap-8', 10: 'gap-10', 12: 'gap-12', 16: 'gap-16'
  },
  sm: {
    0: 'sm:gap-0', 1: 'sm:gap-1', 2: 'sm:gap-2', 3: 'sm:gap-3', 4: 'sm:gap-4', 5: 'sm:gap-5', 6: 'sm:gap-6',
    8: 'sm:gap-8', 10: 'sm:gap-10', 12: 'sm:gap-12', 16: 'sm:gap-16'
  },
  md: {
    0: 'md:gap-0', 1: 'md:gap-1', 2: 'md:gap-2', 3: 'md:gap-3', 4: 'md:gap-4', 5: 'md:gap-5', 6: 'md:gap-6',
    8: 'md:gap-8', 10: 'md:gap-10', 12: 'md:gap-12', 16: 'md:gap-16'
  },
  lg: {
    0: 'lg:gap-0', 1: 'lg:gap-1', 2: 'lg:gap-2', 3: 'lg:gap-3', 4: 'lg:gap-4', 5: 'lg:gap-5', 6: 'lg:gap-6',
    8: 'lg:gap-8', 10: 'lg:gap-10', 12: 'lg:gap-12', 16: 'lg:gap-16'
  },
  xl: {
    0: 'xl:gap-0', 1: 'xl:gap-1', 2: 'xl:gap-2', 3: 'xl:gap-3', 4: 'xl:gap-4', 5: 'xl:gap-5', 6: 'xl:gap-6',
    8: 'xl:gap-8', 10: 'xl:gap-10', 12: 'xl:gap-12', 16: 'xl:gap-16'
  }
}

const getMappedClass = (
  value: number | ResponsiveGridValue | undefined,
  map: Record<string, Record<number, string>>,
) => {
  if (value === undefined) {
    return ''
  }

  if (typeof value === 'number') {
    return map.xs[value] || ''
  }

  const xsClass = value.xs !== undefined ? (map.xs[value.xs] || '') : ''
  const smClass = value.sm !== undefined ? (map.sm[value.sm] || '') : ''
  const mdClass = value.md !== undefined ? (map.md[value.md] || '') : ''
  const lgClass = value.lg !== undefined ? (map.lg[value.lg] || '') : ''
  const xlClass = value.xl !== undefined ? (map.xl[value.xl] || '') : ''

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
      const gapClass = getMappedClass(spacing, GAP_MAP)
      const colsClass =
        getMappedClass(columns ?? 12, GRID_COLS_MAP) || 'grid-cols-12'

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
      const colClasses = getMappedClass(
        { xs, sm, md, lg, xl },
        COL_SPAN_MAP,
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
