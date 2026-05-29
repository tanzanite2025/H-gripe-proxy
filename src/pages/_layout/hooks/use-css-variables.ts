/**
 * Hook to manage CSS variables for theming
 * Replaces the CSS variable logic from use-custom-theme.ts
 */

import { useEffect, useMemo } from 'react'

import { useVerge } from '@/hooks/system'
import { defaultDarkTheme, defaultTheme } from '@/pages/_core/theme'
import { useThemeMode } from '@/services/states'
import { darken } from '@/utils/misc/color'
import { applyCssVariables, updateGradientColors } from '@/utils/theme/css-variables'

export const useCssVariables = () => {
  const mode = useThemeMode()
  const { verge } = useVerge()
  const { theme_setting } = verge ?? {}
  const setting = useMemo(() => theme_setting ?? {}, [theme_setting])
  const dt = useMemo(() => (mode === 'light' ? defaultTheme : defaultDarkTheme), [mode])

  // Apply CSS variables when theme changes
  useEffect(() => {
    if (!mode) return

    applyCssVariables(mode, setting, dt)
  }, [mode, setting, dt])

  // Update gradient colors
  useEffect(() => {
    const primaryColor = setting.primary_color || dt.primary_color
    const primaryDark = darken(primaryColor, 0.2)

    const timerId = setTimeout(() => {
      updateGradientColors(primaryColor, primaryDark)
    }, 0)

    return () => clearTimeout(timerId)
  }, [setting, dt])
}
