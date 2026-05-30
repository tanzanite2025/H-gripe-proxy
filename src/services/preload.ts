import { getVergeConfig } from './cmds'
import {
  cacheLanguage,
  getCachedLanguage,
  resolveLanguage,
} from './i18n'

let vergeConfigCache: IVergeConfig | null | undefined

export const setPreloadConfig = (config: IVergeConfig | null) => {
  vergeConfigCache = config
}

export const getPreloadConfig = () => vergeConfigCache

export const preloadConfig = async () => {
  try {
    const config = await getVergeConfig()
    setPreloadConfig(config)
    return config
  } catch (error) {
    console.warn('[preload.ts] Failed to read Verge config:', error)
    setPreloadConfig(null)
    return null
  }
}

export const preloadLanguage = async (
  vergeConfig?: IVergeConfig | null,
  loadConfig: () => Promise<IVergeConfig | null> = preloadConfig,
) => {
  const cachedLanguage = getCachedLanguage()
  if (cachedLanguage) {
    return cachedLanguage
  }

  let resolvedConfig = vergeConfig

  if (resolvedConfig === undefined) {
    try {
      resolvedConfig = await loadConfig()
    } catch (error) {
      console.warn(
        '[preload.ts] Failed to read language from Verge config:',
        error,
      )
      resolvedConfig = null
    }
  }

  const languageFromConfig = resolvedConfig?.language
  if (languageFromConfig) {
    const resolved = resolveLanguage(languageFromConfig)
    cacheLanguage(resolved)
    return resolved
  }

  const browserLanguage = resolveLanguage(
    typeof navigator !== 'undefined' ? navigator.language : undefined,
  )
  cacheLanguage(browserLanguage)
  return browserLanguage
}
