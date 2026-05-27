import { Select } from '@/components/tailwind'
import type { ComponentProps } from 'react'

type BaseStyledSelectProps = Omit<ComponentProps<typeof Select>, 'options'>

export const BaseStyledSelect = (props: BaseStyledSelectProps & { options: Array<{ value: string | number; label: string }> }) => {
  return (
    <div className="mr-1 w-[120px]">
      <Select {...props} />
    </div>
  )
}
