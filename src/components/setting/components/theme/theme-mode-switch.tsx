import { useTranslation } from 'react-i18next'

import { ButtonGroup, Button } from '@/components/tailwind'

type ThemeValue = IVergeConfig['theme_mode']

interface Props {
  value?: ThemeValue
  onChange?: (value: ThemeValue) => void
}

export const ThemeModeSwitch = (props: Props) => {
  const { value, onChange } = props
  const { t } = useTranslation()

  const modes = ['light', 'dark', 'system'] as const

  return (
    <ButtonGroup className="uds-toolbar my-1" size="small">
      {modes.map((mode) => (
        <Button
          key={mode}
          variant={mode === value ? 'primary' : 'outlined'}
          onClick={() => onChange?.(mode)}
          className="capitalize"
        >
          {t(`settings.sections.appearance.${mode}`)}
        </Button>
      ))}
    </ButtonGroup>
  )
}
