import createCache, { type EmotionCache } from '@emotion/cache'
import { CacheProvider } from '@emotion/react'
import type { PropsWithChildren } from 'react'

const EMOTION_INSERTION_POINT_NAME = 'emotion-insertion-point'
const EMOTION_CACHE_KEY = 'mui'

let emotionStyleCache: EmotionCache | null = null

const ensureEmotionInsertionPoint = () => {
  if (typeof document === 'undefined' || !document.head) return undefined

  const existingInsertionPoint = document.querySelector<HTMLMetaElement>(
    `meta[name="${EMOTION_INSERTION_POINT_NAME}"]`,
  )

  if (existingInsertionPoint) {
    return existingInsertionPoint
  }

  const insertionPoint = document.createElement('meta')
  insertionPoint.setAttribute('name', EMOTION_INSERTION_POINT_NAME)
  insertionPoint.setAttribute('content', '')
  document.head.prepend(insertionPoint)
  return insertionPoint
}

const getEmotionStyleCache = () => {
  if (emotionStyleCache) {
    return emotionStyleCache
  }

  const insertionPoint = ensureEmotionInsertionPoint()
  emotionStyleCache = createCache({
    key: EMOTION_CACHE_KEY,
    insertionPoint,
  })

  const sheet = emotionStyleCache.sheet as typeof emotionStyleCache.sheet & {
    speedy?: (value: boolean) => void
  }

  if (sheet.isSpeedy && typeof sheet.speedy === 'function') {
    sheet.speedy(false)
  }

  return emotionStyleCache
}

export const EmotionStyleChain = ({ children }: PropsWithChildren) => {
  return <CacheProvider value={getEmotionStyleCache()}>{children}</CacheProvider>
}