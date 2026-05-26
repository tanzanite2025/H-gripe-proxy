import { alpha, createTheme, Shadows, type ThemeOptions } from '@mui/material'
import {
  getCurrentWebviewWindow,
  WebviewWindow,
} from '@tauri-apps/api/webviewWindow'
import { Theme as TauriOsTheme } from '@tauri-apps/api/window'
import { useEffect, useLayoutEffect, useMemo } from 'react'

import { useVerge } from '@/hooks/system'
import { defaultDarkTheme, defaultTheme } from '@/pages/_theme'
import { useSetThemeMode, useThemeMode } from '@/services/states'

const CSS_INJECTION_SCOPE_ROOT = '[data-css-injection-root]'
const CSS_INJECTION_SCOPE_LIMIT =
  ':is(.monaco-editor .view-lines, .monaco-editor .view-line, .monaco-editor .margin, .monaco-editor .margin-view-overlays, .monaco-editor .view-overlays, .monaco-editor [class^="mtk"], .monaco-editor [class*=" mtk"])'
const TOP_LEVEL_AT_RULES = [
  '@charset',
  '@import',
  '@namespace',
  '@font-face',
  '@keyframes',
  '@counter-style',
  '@page',
  '@property',
  '@font-feature-values',
  '@color-profile',
]
let cssScopeSupport: boolean | null = null

const canUseCssScope = () => {
  if (cssScopeSupport !== null) {
    return cssScopeSupport
  }
  try {
    const testStyle = document.createElement('style')
    testStyle.textContent = '@scope (:root) { }'
    document.head.appendChild(testStyle)
    cssScopeSupport = !!testStyle.sheet?.cssRules?.length
    document.head.removeChild(testStyle)
  } catch {
    cssScopeSupport = false
  }
  return cssScopeSupport
}

const wrapCssInjectionWithScope = (css?: string) => {
  if (!css?.trim()) {
    return ''
  }
  const lowerCss = css.toLowerCase()
  const hasTopLevelOnlyRule = TOP_LEVEL_AT_RULES.some((rule) =>
    lowerCss.includes(rule),
  )
  if (hasTopLevelOnlyRule) {
    return null
  }
  const scopeRoot = CSS_INJECTION_SCOPE_ROOT
  const scopeLimit = CSS_INJECTION_SCOPE_LIMIT
  const scopedBlock = `@scope (${scopeRoot}) to (${scopeLimit}) {
${css}
}`
  return scopedBlock
}

const hexColorToRgbString = (color: string) => {
  const normalized = color.trim().replace('#', '')

  if (!/^[\da-f]{3,8}$/i.test(normalized)) {
    return null
  }

  const hex =
    normalized.length === 3 || normalized.length === 4
      ? normalized
          .slice(0, 3)
          .split('')
          .map((char) => char + char)
          .join('')
      : normalized.slice(0, 6)

  const r = Number.parseInt(hex.slice(0, 2), 16)
  const g = Number.parseInt(hex.slice(2, 4), 16)
  const b = Number.parseInt(hex.slice(4, 6), 16)

  if ([r, g, b].some(Number.isNaN)) {
    return null
  }

  return `${r}, ${g}, ${b}`
}

const rgbFunctionToRgbString = (color: string) => {
  const channels = color.match(/\d+(?:\.\d+)?/g)

  if (!channels || channels.length < 3) {
    return null
  }

  return channels
    .slice(0, 3)
    .map((channel) => {
      const value = Number.parseFloat(channel)
      return String(Math.max(0, Math.min(255, Math.round(value))))
    })
    .join(', ')
}

const resolveCssColorToRgbString = (
  color: string,
  fallback = '91, 92, 157',
) => {
  const hexRgb = hexColorToRgbString(color)
  if (hexRgb) {
    return hexRgb
  }

  const rawRgb = rgbFunctionToRgbString(color)
  if (rawRgb) {
    return rawRgb
  }

  if (typeof document !== 'undefined') {
    const probe = document.createElement('div')
    probe.style.color = color
    probe.style.position = 'absolute'
    probe.style.visibility = 'hidden'
    probe.style.pointerEvents = 'none'
    document.documentElement.appendChild(probe)

    const computedColor = window.getComputedStyle(probe).color

    document.documentElement.removeChild(probe)

    const computedRgb = rgbFunctionToRgbString(computedColor)
    if (computedRgb) {
      return computedRgb
    }
  }

  return fallback
}

/**
 * custom theme
 */
export const useCustomTheme = () => {
  const appWindow: WebviewWindow = useMemo(() => getCurrentWebviewWindow(), [])
  const { verge } = useVerge()
  const { theme_mode, theme_setting } = verge ?? {}
  const mode = useThemeMode()
  const setMode = useSetThemeMode()
  const setting = useMemo(() => theme_setting || {}, [theme_setting])
  const dt = useMemo(
    () => (mode === 'light' ? defaultTheme : defaultDarkTheme),
    [mode],
  )
  const userBackgroundImage = setting.background_image || ''
  const hasUserBackground = !!userBackgroundImage

  useEffect(() => {
    if (theme_mode === 'light' || theme_mode === 'dark') {
      setMode(theme_mode)
    }
  }, [theme_mode, setMode])

  useEffect(() => {
    if (theme_mode !== 'system') {
      return
    }

    let isMounted = true

    const timerId = setTimeout(() => {
      if (!isMounted) return
      appWindow
        .theme()
        .then((systemTheme) => {
          if (isMounted && systemTheme) {
            setMode(systemTheme)
          }
        })
        .catch((err) => {
          console.error('Failed to get initial system theme:', err)
        })
    }, 0)

    const unlistenPromise = appWindow.onThemeChanged(({ payload }) => {
      if (isMounted) {
        setMode(payload)
      }
    })

    return () => {
      isMounted = false
      clearTimeout(timerId)
      unlistenPromise
        .then((unlistenFn) => {
          if (typeof unlistenFn === 'function') {
            unlistenFn()
          }
        })
        .catch((err) => {
          console.error('Failed to unlisten from theme changes:', err)
        })
    }
  }, [theme_mode, appWindow, setMode])

  useEffect(() => {
    if (theme_mode === undefined) {
      return
    }

    if (theme_mode === 'system') {
      appWindow.setTheme(null).catch((err) => {
        console.error(
          'Failed to set window theme to follow system (setTheme(null)):',
          err,
        )
      })
    } else if (mode) {
      appWindow.setTheme(mode as TauriOsTheme).catch((err) => {
        console.error(`Failed to set window theme to ${mode}:`, err)
      })
    }
  }, [mode, appWindow, theme_mode])

  const theme = useMemo(() => {
    const resolvedPrimaryColor = setting.primary_color || dt.primary_color
    const resolvedSecondaryColor = setting.secondary_color || dt.secondary_color
    const resolvedPrimaryText = setting.primary_text || dt.primary_text
    const resolvedSecondaryText = setting.secondary_text || dt.secondary_text
    const resolvedFontFamily = setting.font_family || dt.font_family
    const flatShadows = Array(25).fill('none') as Shadows

    const createThemeOptions = (): ThemeOptions => {
      const themeOptions: ThemeOptions = {
        breakpoints: {
          values: { xs: 0, sm: 650, md: 900, lg: 1200, xl: 1536 },
        },
        palette: {
          mode,
          primary: {
            main: resolvedPrimaryColor,
          },
          secondary: {
            main: resolvedSecondaryColor,
          },
          text: {
            primary: resolvedPrimaryText,
            secondary: resolvedSecondaryText,
          },
          background: {
            paper: dt.background_color,
            default: dt.background_color,
          },
        },
        typography: {
          fontFamily: resolvedFontFamily,
          h1: {
            fontSize: '18px',
            fontWeight: 900,
            fontStyle: 'italic',
            letterSpacing: '-0.05em',
            textTransform: 'uppercase',
          },
          h2: {
            fontSize: '14px',
            fontWeight: 900,
            fontStyle: 'italic',
            letterSpacing: '-0.05em',
          },
          h3: {
            fontSize: '14px',
            fontWeight: 900,
            fontStyle: 'italic',
            letterSpacing: '-0.05em',
          },
          h4: {
            fontSize: '14px',
            fontWeight: 900,
            fontStyle: 'italic',
            letterSpacing: '-0.05em',
          },
          h5: {
            fontSize: '14px',
            fontWeight: 900,
            fontStyle: 'italic',
            letterSpacing: '-0.05em',
          },
          h6: {
            fontSize: '14px',
            fontWeight: 900,
            fontStyle: 'italic',
            letterSpacing: '-0.05em',
          },
          body1: {
            fontSize: '12px',
            fontWeight: 500,
          },
          body2: {
            fontSize: '12px',
            fontWeight: 500,
          },
          caption: {
            fontSize: '10px',
            fontWeight: 900,
            letterSpacing: '0.15em',
            textTransform: 'uppercase',
          },
        },
        components: {
          MuiButton: {
            defaultProps: {
              disableElevation: true,
            },
            styleOverrides: {
              root: {
                borderRadius: '9999px',
                textTransform: 'uppercase',
                fontWeight: 900,
                letterSpacing: '0.15em',
                fontSize: '10px',
                height: '44px',
                padding: '0 24px',
                transition: 'all 0.25s cubic-bezier(0.16, 1, 0.3, 1)',
              },
              contained: {
                backgroundColor: resolvedPrimaryColor,
                color: '#ffffff',
                '&:hover': {
                  backgroundColor: resolvedPrimaryColor,
                  opacity: 0.9,
                  transform: 'translateY(-1px)',
                  boxShadow: '0 4px 12px rgba(0,0,0,0.15)',
                },
              },
              outlined: {
                border:
                  mode === 'light'
                    ? '1px dashed rgba(0, 0, 0, 0.15)'
                    : '1px dashed rgba(255, 255, 255, 0.15)',
                backgroundColor: 'transparent',
                color: mode === 'light' ? '#111827' : '#ffffff',
                '&:hover': {
                  border:
                    mode === 'light'
                      ? '1px dashed rgba(0, 0, 0, 0.3)'
                      : '1px dashed rgba(255, 255, 255, 0.3)',
                  backgroundColor:
                    mode === 'light'
                      ? 'rgba(0, 0, 0, 0.03)'
                      : 'rgba(255, 255, 255, 0.05)',
                  transform: 'translateY(-1px)',
                },
              },
            },
          },
          MuiOutlinedInput: {
            styleOverrides: {
              root: {
                borderRadius: '16px',
                height: '48px',
                backgroundColor:
                  mode === 'light'
                    ? 'rgba(0, 0, 0, 0.03)'
                    : 'rgba(255, 255, 255, 0.04)',
                transition: 'all 0.25s cubic-bezier(0.16, 1, 0.3, 1)',
                fontSize: '12px',
                fontWeight: 600,
                '& .MuiOutlinedInput-notchedOutline': {
                  border: 'none',
                },
                '&:hover': {
                  backgroundColor:
                    mode === 'light'
                      ? 'rgba(0, 0, 0, 0.05)'
                      : 'rgba(255, 255, 255, 0.07)',
                },
                '&.Mui-focused': {
                  backgroundColor:
                    mode === 'light'
                      ? 'rgba(0, 0, 0, 0.02)'
                      : 'rgba(255, 255, 255, 0.02)',
                  boxShadow: `0 0 0 2px ${alpha(resolvedPrimaryColor, 0.2)}`,
                },
              },
              input: {
                padding: '0 16px',
                height: '100%',
                display: 'flex',
                alignItems: 'center',
              },
            },
          },
          MuiInputBase: {
            styleOverrides: {
              root: {
                borderRadius: '16px',
                fontSize: '12px',
                fontWeight: 600,
              },
            },
          },
          MuiSelect: {
            styleOverrides: {
              select: {
                height: '48px',
                display: 'flex',
                alignItems: 'center',
                boxSizing: 'border-box',
                paddingLeft: '16px',
                paddingRight: '32px',
              },
            },
          },
          MuiDialog: {
            styleOverrides: {
              paper: {
                borderRadius: '32px',
                border: 'none',
                boxShadow:
                  mode === 'light'
                    ? '0 25px 50px -12px rgba(0, 0, 0, 0.15)'
                    : '0 25px 50px -12px rgba(0, 0, 0, 0.5)',
                backgroundImage: 'none',
              },
            },
          },
          MuiDialogTitle: {
            styleOverrides: {
              root: {
                padding: '24px 24px 12px',
                fontSize: '14px',
                fontWeight: 900,
                fontStyle: 'italic',
                letterSpacing: '-0.05em',
              },
            },
          },
          MuiDialogContent: {
            styleOverrides: {
              root: {
                padding: '12px 24px 20px',
              },
            },
          },
          MuiDialogActions: {
            styleOverrides: {
              root: {
                padding: '0 24px 24px',
                gap: '8px',
              },
            },
          },
          MuiMenu: {
            styleOverrides: {
              paper: {
                borderRadius: '16px',
                boxShadow:
                  mode === 'light'
                    ? '0 10px 25px -5px rgba(0, 0, 0, 0.08), 0 8px 10px -6px rgba(0, 0, 0, 0.04)'
                    : '0 10px 25px -5px rgba(0, 0, 0, 0.4), 0 8px 10px -6px rgba(0, 0, 0, 0.3)',
                border:
                  mode === 'light'
                    ? '1px solid rgba(0, 0, 0, 0.06)'
                    : '1px solid rgba(255, 255, 255, 0.04)',
              },
            },
          },
          MuiPopover: {
            styleOverrides: {
              paper: {
                borderRadius: '16px',
                boxShadow:
                  mode === 'light'
                    ? '0 10px 25px -5px rgba(0, 0, 0, 0.08)'
                    : '0 10px 25px -5px rgba(0, 0, 0, 0.4)',
                border:
                  mode === 'light'
                    ? '1px solid rgba(0, 0, 0, 0.06)'
                    : '1px solid rgba(255, 255, 255, 0.04)',
              },
            },
          },
          MuiTooltip: {
            styleOverrides: {
              tooltip: {
                borderRadius: '8px',
                fontSize: '10px',
                fontWeight: 900,
                backgroundColor: '#111827',
                color: '#ffffff',
                letterSpacing: '0.05em',
                padding: '6px 10px',
              },
              arrow: {
                color: '#111827',
              },
            },
          },
          MuiFormControlLabel: {
            styleOverrides: {
              root: {
                marginLeft: 0,
                marginRight: 0,
                padding: '4px 10px',
                borderRadius: '12px',
                transition: 'background-color 0.2s ease',
                '&:hover': {
                  backgroundColor:
                    mode === 'light'
                      ? 'rgba(0, 0, 0, 0.03)'
                      : 'rgba(255, 255, 255, 0.03)',
                },
              },
            },
          },
        },
      }

      themeOptions.shadows = flatShadows

      return themeOptions
    }

    return createTheme(createThemeOptions())
  }, [dt, mode, setting])

  useLayoutEffect(() => {
    const rootEle = document.documentElement
    if (rootEle) {
      const backgroundColor = dt.background_color
      const selectColor = mode === 'light' ? '#f5f5f5' : '#3E3E3E'
      const scrollColor = mode === 'light' ? '#90939980' : '#555555'
      const dividerColor =
        mode === 'light' ? 'rgba(0, 0, 0, 0.06)' : 'rgba(255, 255, 255, 0.04)' // 极淡灰色虚线
      
      // 设置字体 CSS 变量（唯一数据源）
      const resolvedFontFamily = setting.font_family || dt.font_family
      rootEle.style.setProperty('--font-family', resolvedFontFamily)
      
      rootEle.style.setProperty('--divider-color', dividerColor)
      rootEle.style.setProperty('--background-color', backgroundColor)
      rootEle.style.setProperty('--selection-color', selectColor)
      rootEle.style.setProperty('--scroller-color', scrollColor)
      rootEle.style.setProperty('--primary-main', theme.palette.primary.main)

      // 计算 RGB 变量以供 SCSS 渐变使用
      const primaryRgb = resolveCssColorToRgbString(theme.palette.primary.main)
      rootEle.style.setProperty('--primary-main-rgb', primaryRgb)

      rootEle.style.setProperty(
        '--background-color-alpha',
        alpha(theme.palette.primary.main, 0.1),
      )
      rootEle.style.setProperty(
        '--window-border-color',
        mode === 'light' ? '#cccccc' : '#1E1E1E',
      )
      rootEle.style.setProperty(
        '--scrollbar-bg',
        mode === 'light' ? '#f1f1f1' : '#2E303D',
      )
      rootEle.style.setProperty(
        '--scrollbar-thumb',
        mode === 'light' ? '#c1c1c1' : '#555555',
      )

      // 浅色模式卡片为纯白 (#ffffff)，深色模式为钛金黑 (#16181d)
      rootEle.style.setProperty(
        '--card-bg',
        mode === 'light' ? '#ffffff' : '#16181d',
      )
      rootEle.style.setProperty('--text-primary', theme.palette.text.primary)
      rootEle.style.setProperty(
        '--text-secondary',
        theme.palette.text.secondary,
      )
      rootEle.style.setProperty(
        '--primary-main-hover',
        theme.palette.primary.main,
      )
      rootEle.style.setProperty(
        '--layout-nav-active-bg',
        alpha(theme.palette.primary.main, mode === 'light' ? 0.15 : 0.35),
      )
      rootEle.style.setProperty(
        '--user-background-image',
        hasUserBackground ? `url('${userBackgroundImage}')` : 'none',
      )
      rootEle.style.setProperty(
        '--background-blend-mode',
        setting.background_blend_mode || 'normal',
      )
      rootEle.style.setProperty(
        '--background-opacity',
        setting.background_opacity !== undefined
          ? String(setting.background_opacity)
          : '1',
      )
      rootEle.setAttribute('data-css-injection-root', 'true')
      rootEle.setAttribute('data-theme', mode)
    }

    let styleElement =
      document.querySelector<HTMLStyleElement>('style#verge-theme')
    if (!styleElement) {
      styleElement = document.createElement('style')
      styleElement.id = 'verge-theme'
      document.head.appendChild(styleElement)
    }

    let scopedCss: string | null = null
    if (canUseCssScope() && setting.css_injection) {
      scopedCss = wrapCssInjectionWithScope(setting.css_injection)
    }
    const effectiveInjectedCss = scopedCss ?? setting.css_injection ?? ''
    const globalStyles = `
        /* 修复滚动条样式 */
        ::-webkit-scrollbar {
          width: 8px;
          height: 8px;
          background-color: var(--scrollbar-bg);
        }
        ::-webkit-scrollbar-thumb {
          background-color: var(--scrollbar-thumb);
          border-radius: 4px;
        }
        ::-webkit-scrollbar-thumb:hover {
          background-color: ${mode === 'light' ? '#a1a1a1' : '#666666'};
        }

        /* 背景图处理 */
        body {
          background-color: var(--background-color);
          ${
            hasUserBackground
              ? `
            background-image: var(--user-background-image);
            background-size: cover;
            background-position: center;
            background-attachment: fixed;
            background-blend-mode: var(--background-blend-mode);
            opacity: var(--background-opacity);
          `
              : ''
          }
        }

        /* 移除可能的白色点或线条 */
        * {
          outline: none !important;
          box-shadow: none !important;
        }
      `

    styleElement.textContent = effectiveInjectedCss + globalStyles
  }, [dt, hasUserBackground, mode, setting, theme, userBackgroundImage])

  useEffect(() => {
    const id = setTimeout(() => {
      const dom = document.querySelector('#Gradient2')
      if (dom) {
        dom.innerHTML = `
        <stop offset="0%" stop-color="${theme.palette.primary.main}" />
        <stop offset="80%" stop-color="${theme.palette.primary.dark}" />
        <stop offset="100%" stop-color="${theme.palette.primary.dark}" />
        `
      }
    }, 0)
    return () => clearTimeout(id)
  }, [theme.palette.primary.main, theme.palette.primary.dark])

  return { theme }
}
