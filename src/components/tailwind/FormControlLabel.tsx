import { forwardRef, type ReactElement } from 'react'

import { cn } from '@/utils/cn'

export interface FormControlLabelProps {
  control: ReactElement<any, any>
  label: string | ReactElement
  className?: string
  disabled?: boolean
  labelPlacement?: 'end' | 'start' | 'top' | 'bottom'
}

export const FormControlLabel = forwardRef<HTMLLabelElement, FormControlLabelProps>(
  ({ control, label, className, disabled, labelPlacement = 'end' }, ref) => {
    const flexDirection = {
      end: 'flex-row',
      start: 'flex-row-reverse',
      top: 'flex-col-reverse',
      bottom: 'flex-col',
    }

    const ControlComponent = control.type as React.ComponentType<any>

    return (
      <label
        ref={ref}
        className={cn(
          'inline-flex items-center gap-2 cursor-pointer',
          flexDirection[labelPlacement],
          disabled && 'opacity-50 cursor-not-allowed',
          className
        )}
      >
        <ControlComponent {...control.props} disabled={disabled} />
        <span className="text-sm select-none">{label}</span>
      </label>
    )
  }
)

FormControlLabel.displayName = 'FormControlLabel'
