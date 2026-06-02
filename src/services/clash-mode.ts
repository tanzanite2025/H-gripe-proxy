export const CLASH_MODES = ['rule', 'global', 'direct'] as const

export type ClashMode = (typeof CLASH_MODES)[number]

export const DEFAULT_CLASH_MODE: ClashMode = 'rule'

const CLASH_MODE_SET = new Set<string>(CLASH_MODES)

export const isClashMode = (value: unknown): value is ClashMode =>
  typeof value === 'string' && CLASH_MODE_SET.has(value)

export const normalizeClashMode = (value: unknown): ClashMode | undefined => {
  if (typeof value !== 'string') return undefined

  const normalized = value.trim().toLowerCase()
  return isClashMode(normalized) ? normalized : undefined
}

export const resolveClashMode = (
  primary: unknown,
  fallback?: unknown,
): ClashMode | undefined => normalizeClashMode(primary) ?? normalizeClashMode(fallback)
