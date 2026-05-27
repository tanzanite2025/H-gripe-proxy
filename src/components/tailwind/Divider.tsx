import { type HTMLAttributes } from 'react'

export interface DividerProps extends HTMLAttributes<HTMLHRElement> {
  orientation?: 'horizontal' | 'vertical'
}

export const Divider = ({
  orientation = 'horizontal',
  className = '',
  ...props
}: DividerProps) => {
  const orientationClasses =
    orientation === 'horizontal'
      ? 'w-full h-px'
      : 'h-full w-px'

  return (
    <hr
      className={`border-0 bg-divider-light dark:bg-divider-dark ${orientationClasses} ${className}`}
      {...props}
    />
  )
}

Divider.displayName = 'Divider'
