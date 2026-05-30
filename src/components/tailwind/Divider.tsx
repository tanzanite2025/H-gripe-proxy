import { type HTMLAttributes } from 'react'

export interface DividerProps extends HTMLAttributes<HTMLHRElement> {
  orientation?: 'horizontal' | 'vertical'
  variant?: 'fullWidth' | 'inset' | 'middle'
  flexItem?: boolean
}

export const Divider = ({
  orientation = 'horizontal',
  variant = 'fullWidth',
  flexItem = false,
  className = '',
  ...props
}: DividerProps) => {
  const orientationClasses =
    orientation === 'horizontal'
      ? 'w-full h-px'
      : 'h-full w-px'

  const variantClasses = {
    fullWidth: '',
    inset: orientation === 'horizontal' ? 'ml-16' : 'mt-16',
    middle: orientation === 'horizontal' ? 'mx-4' : 'my-4',
  }

  const flexItemClass = flexItem ? 'self-stretch' : ''

  return (
    <hr
      className={`border-0 bg-divider ${orientationClasses} ${variantClasses[variant]} ${flexItemClass} ${className}`}
      {...props}
    />
  )
}

Divider.displayName = 'Divider'
