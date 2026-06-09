export function normalizePolicyName(value?: string | null): string {
  return typeof value === 'string' ? value.trim() : ''
}

export function categorizeDelay(
  delay: number,
  effectiveTimeout: number,
): [number, number] {
  if (!Number.isFinite(delay)) return [5, Number.MAX_SAFE_INTEGER]
  if (delay > 1e5) return [4, delay]

  if (delay === 0 || (delay >= effectiveTimeout && delay <= 1e5)) {
    return [3, delay || effectiveTimeout]
  }

  if (delay < 0) return [5, Number.MAX_SAFE_INTEGER]
  return [0, delay]
}
