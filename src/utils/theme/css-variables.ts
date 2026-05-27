/**
 * CSS Variables Management
 * 
 * This utility manages CSS custom properties (variables) that are used throughout the application.
 * Previously part of use-custom-theme.ts, now extracted for use with Tailwind CSS.
 */

import { alpha } from '@/utils/misc/color'

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

export interface ThemeSettings {
  primary_color?: string
  secondary_color?: string
  primary_text?: string
  secondary_text?: string
  font_family?: string
  background_image?: string
  background_blend_mode?: string
  background_opacity?: number
  css_injection?: string
}

export interface DefaultTheme {
  primary_color: string
  secondary_color: string
  primary_text: string
  secondary_text: string
  font_family: string
  background_color: string
}

/**
 * Apply CSS variables to the document root
 */
export const applyCssVariables = (
  mode: 'light' | 'dark',
  settings: ThemeSettings,
  defaultTheme: DefaultTheme,
) => {
  const rootEle = document.documentElement
  if (!rootEle) return

  const backgroundColor = defaultTheme.background_color
  const selectColor = mode === 'light' ? '#f5f5f5' : '#3E3E3E'
  const scrollColor = mode === 'light' ? '#90939980' : '#555555'
  const dividerColor =
    mode === 'light' ? 'rgba(0, 0, 0, 0.06)' : 'rgba(255, 255, 255, 0.04)'

  // Font family
  const resolvedFontFamily = settings.font_family || defaultTheme.font_family
  rootEle.style.setProperty('--font-family', resolvedFontFamily)

  // Colors
  rootEle.style.setProperty('--divider-color', dividerColor)
  rootEle.style.setProperty('--background-color', backgroundColor)
  rootEle.style.setProperty('--selection-color', selectColor)
  rootEle.style.setProperty('--scroller-color', scrollColor)

  // Primary color
  const primaryColor = settings.primary_color || defaultTheme.primary_color
  rootEle.style.setProperty('--primary-main', primaryColor)

  // Primary RGB for gradients
  const primaryRgb = resolveCssColorToRgbString(primaryColor)
  rootEle.style.setProperty('--primary-main-rgb', primaryRgb)

  // Alpha colors
  rootEle.style.setProperty(
    '--background-color-alpha',
    alpha(primaryColor, 0.1),
  )

  // Window and scrollbar
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

  // Card background
  rootEle.style.setProperty(
    '--card-bg',
    mode === 'light' ? '#ffffff' : '#16181d',
  )

  // Text colors
  const primaryText = settings.primary_text || defaultTheme.primary_text
  const secondaryText = settings.secondary_text || defaultTheme.secondary_text
  rootEle.style.setProperty('--text-primary', primaryText)
  rootEle.style.setProperty('--text-secondary', secondaryText)

  // Hover and active states
  rootEle.style.setProperty('--primary-main-hover', primaryColor)
  rootEle.style.setProperty(
    '--layout-nav-active-bg',
    alpha(primaryColor, mode === 'light' ? 0.15 : 0.35),
  )

  // Background image
  const hasUserBackground = !!settings.background_image
  rootEle.style.setProperty(
    '--user-background-image',
    hasUserBackground ? `url('${settings.background_image}')` : 'none',
  )
  rootEle.style.setProperty(
    '--background-blend-mode',
    settings.background_blend_mode || 'normal',
  )
  rootEle.style.setProperty(
    '--background-opacity',
    settings.background_opacity !== undefined
      ? String(settings.background_opacity)
      : '1',
  )

  // Data attributes
  rootEle.setAttribute('data-css-injection-root', 'true')
  rootEle.setAttribute('data-theme', mode)

  // Custom CSS injection
  let styleElement =
    document.querySelector<HTMLStyleElement>('style#verge-theme')
  if (!styleElement) {
    styleElement = document.createElement('style')
    styleElement.id = 'verge-theme'
    document.head.appendChild(styleElement)
  }

  let scopedCss: string | null = null
  if (canUseCssScope() && settings.css_injection) {
    scopedCss = wrapCssInjectionWithScope(settings.css_injection)
  }
  const effectiveInjectedCss = scopedCss ?? settings.css_injection ?? ''
  const globalStyles = `
    /* Scrollbar styles */
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

    /* Background image */
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

    /* Remove white dots or lines */
    * {
      outline: none !important;
      box-shadow: none !important;
    }
  `

  styleElement.textContent = effectiveInjectedCss + globalStyles
}

/**
 * Update gradient SVG colors
 */
export const updateGradientColors = (primaryColor: string, primaryDark: string) => {
  const dom = document.querySelector('#Gradient2')
  if (dom) {
    dom.innerHTML = `
      <stop offset="0%" stop-color="${primaryColor}" />
      <stop offset="80%" stop-color="${primaryDark}" />
      <stop offset="100%" stop-color="${primaryDark}" />
    `
  }
}
