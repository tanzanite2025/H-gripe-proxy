import { Switch as TailwindSwitch } from '@/components/tailwind'
import type { ComponentProps } from 'react'

export const Switch = (props: ComponentProps<typeof TailwindSwitch>) => {
  return <TailwindSwitch {...props} />
}
