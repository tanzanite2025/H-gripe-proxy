import React from 'react'
import { cn } from '@/utils/cn'

type Props = {
  label: string
  fontSize?: string
  width?: string
  padding?: string
  className?: string
  children?: React.ReactNode
}

export const BaseFieldset: React.FC<Props> = ({
  label,
  fontSize = '1em',
  width = 'auto',
  padding = '15px',
  className,
  children,
}: Props) => {
  return (
    <fieldset
      className={cn(
        'relative rounded-md border border-gray-400 dark:border-gray-600',
        className
      )}
      style={{ width, padding }}
    >
      <legend
        className="absolute -top-2.5 bg-card-light px-2 text-gray-900 dark:bg-card-dark dark:text-gray-100"
        style={{ fontSize, left: padding }}
      >
        {label}
      </legend>
      {children}
    </fieldset>
  )
}
