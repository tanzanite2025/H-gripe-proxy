export const DEFAULT_HONEYPOT_DECOY_ID = 'default-config-decoy'

export interface HoneypotDecoy {
  id: string
  name: string
  path: string
  kind: 'config-file'
  enabled: boolean
}

export interface NewHoneypotDecoyInput {
  name: string
  path: string
  kind?: HoneypotDecoy['kind']
  enabled?: boolean
}

export function createDefaultHoneypotDecoys(): HoneypotDecoy[] {
  return [
    {
      id: DEFAULT_HONEYPOT_DECOY_ID,
      name: '默认配置诱饵',
      path: 'config_decoy.yaml',
      kind: 'config-file',
      enabled: true,
    },
  ]
}

export function getActiveHoneypotDecoyPath(
  decoys: HoneypotDecoy[],
  activeDecoyId: string,
): string {
  return decoys.find((decoy) => decoy.id === activeDecoyId)?.path ?? ''
}

export function updateActiveHoneypotDecoyPath(
  decoys: HoneypotDecoy[],
  activeDecoyId: string,
  path: string,
): HoneypotDecoy[] {
  return decoys.map((decoy) =>
    decoy.id === activeDecoyId ? { ...decoy, path } : decoy,
  )
}

export function selectActiveHoneypotDecoyId(
  decoys: HoneypotDecoy[],
  requestedDecoyId: string,
  fallbackDecoyId = DEFAULT_HONEYPOT_DECOY_ID,
): string {
  if (decoys.some((decoy) => decoy.id === requestedDecoyId)) {
    return requestedDecoyId
  }

  if (decoys.some((decoy) => decoy.id === fallbackDecoyId)) {
    return fallbackDecoyId
  }

  return decoys[0]?.id ?? ''
}

export function addHoneypotDecoy(
  decoys: HoneypotDecoy[],
  input: NewHoneypotDecoyInput,
): HoneypotDecoy[] {
  const baseId = createHoneypotDecoyId(input.name || input.path || 'decoy')
  const id = createUniqueHoneypotDecoyId(decoys, baseId)

  return [
    ...decoys,
    {
      id,
      name: input.name,
      path: input.path,
      kind: input.kind ?? 'config-file',
      enabled: input.enabled ?? true,
    },
  ]
}

export function removeHoneypotDecoy(
  decoys: HoneypotDecoy[],
  decoyId: string,
): HoneypotDecoy[] {
  if (decoys.length <= 1) {
    return decoys
  }

  return decoys.filter((decoy) => decoy.id !== decoyId)
}

export function setHoneypotDecoyEnabled(
  decoys: HoneypotDecoy[],
  decoyId: string,
  enabled: boolean,
): HoneypotDecoy[] {
  if (!enabled && getEnabledHoneypotDecoys(decoys).length <= 1) {
    return decoys
  }

  return decoys.map((decoy) =>
    decoy.id === decoyId ? { ...decoy, enabled } : decoy,
  )
}

export function getEnabledHoneypotDecoys(decoys: HoneypotDecoy[]): HoneypotDecoy[] {
  return decoys.filter((decoy) => decoy.enabled)
}

export function normalizeActiveHoneypotDecoyId(
  decoys: HoneypotDecoy[],
  activeDecoyId: string,
): string {
  const activeDecoy = decoys.find((decoy) => decoy.id === activeDecoyId)
  if (activeDecoy?.enabled) {
    return activeDecoyId
  }

  const enabledDecoys = getEnabledHoneypotDecoys(decoys)
  if (enabledDecoys.length > 0) {
    return selectActiveHoneypotDecoyId(enabledDecoys, activeDecoyId)
  }

  return selectActiveHoneypotDecoyId(decoys, activeDecoyId)
}

function createHoneypotDecoyId(value: string): string {
  const normalized = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')

  return normalized ? `decoy-${normalized}` : 'decoy-config'
}

function createUniqueHoneypotDecoyId(
  decoys: HoneypotDecoy[],
  baseId: string,
): string {
  const existingIds = new Set(decoys.map((decoy) => decoy.id))
  let nextId = baseId
  let suffix = 2

  while (existingIds.has(nextId)) {
    nextId = `${baseId}-${suffix}`
    suffix += 1
  }

  return nextId
}
