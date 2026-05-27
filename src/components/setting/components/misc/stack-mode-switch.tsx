import { Button, ButtonGroup } from '@/components/tailwind'

interface Props {
  value?: string
  onChange?: (value: string) => void
}

export const StackModeSwitch = (props: Props) => {
  const { value, onChange } = props

  return (
    <ButtonGroup className="uds-toolbar my-1" size="small">
      <Button
        variant={value?.toLowerCase() === 'system' ? 'primary' : 'outlined'}
        onClick={() => onChange?.('system')}
        className="capitalize"
      >
        System
      </Button>
      <Button
        variant={value?.toLowerCase() === 'gvisor' ? 'primary' : 'outlined'}
        onClick={() => onChange?.('gvisor')}
        className="capitalize"
      >
        gVisor
      </Button>
      <Button
        variant={value?.toLowerCase() === 'mixed' ? 'primary' : 'outlined'}
        onClick={() => onChange?.('mixed')}
        className="capitalize"
      >
        Mixed
      </Button>
    </ButtonGroup>
  )
}
