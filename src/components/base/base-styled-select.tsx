import { Select } from '@/components/tailwind'
import type { NativeSelectProps } from '@/components/tailwind/Select'

type BaseStyledSelectProps = NativeSelectProps

export const BaseStyledSelect = (props: BaseStyledSelectProps) => {
  return (
    <div className="mr-1 w-[120px]">
      <Select {...props} />
    </div>
  )
}
