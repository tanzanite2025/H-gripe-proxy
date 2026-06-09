const HIDDEN_PROXY_NAMES = new Set(['COMPATIBLE'])
const BUILTIN_POLICY_NAMES = new Set(['DIRECT', 'REJECT', 'REJECT-DROP', 'PASS'])

export const normalizeType = (type?: string) => type?.trim().toLowerCase() || ''

export const normalizeName = (value?: string | null) => value?.trim() || ''

export const isHiddenProxyName = (name?: string | null) =>
  HIDDEN_PROXY_NAMES.has(normalizeName(name).toUpperCase())

export const isBuiltinPolicyName = (name?: string | null) =>
  BUILTIN_POLICY_NAMES.has(normalizeName(name).toUpperCase())

export const collectProxyNames = (
  group?: { all?: Array<string | { name?: string }> } | null,
): string[] =>
  Array.isArray(group?.all)
    ? group.all
        .map((item) =>
          typeof item === 'string'
            ? normalizeName(item)
            : normalizeName(item?.name),
        )
        .filter(
          (name) =>
            name.length > 0 &&
            !isHiddenProxyName(name) &&
            !isBuiltinPolicyName(name),
        )
    : []

