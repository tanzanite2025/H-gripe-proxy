import type { ComponentProps } from 'react'

import { Switch as TailwindSwitch } from '@/components/tailwind'

export const Switch = (props: ComponentProps<typeof TailwindSwitch>) => {
  return <TailwindSwitch {...props} />
}
