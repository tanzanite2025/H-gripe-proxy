import yaml from 'js-yaml'

/**
 * Normalize delete sequence from various input formats
 */
export const normalizeDeleteSeq = (input?: unknown): string[] => {
  if (!Array.isArray(input)) {
    return []
  }

  const names = input
    .map((item) => {
      if (typeof item === 'string') {
        return item
      }

      if (
        item &&
        typeof item === 'object' &&
        'name' in item &&
        typeof (item as { name: unknown }).name === 'string'
      ) {
        return (item as { name: string }).name
      }

      return undefined
    })
    .filter(
      (name): name is string => typeof name === 'string' && name.length > 0,
    )

  return Array.from(new Set(names))
}

/**
 * Build YAML string from group sequences
 */
export const buildGroupsYaml = (
  prepend: IProxyGroupConfig[],
  append: IProxyGroupConfig[],
  deleteList: string[],
) => {
  return yaml.dump(
    {
      prepend,
      append,
      delete: deleteList,
    },
    { forceQuotes: true },
  )
}

/**
 * Parse YAML string to group sequences
 */
export const parseGroupsYaml = (data: string) => {
  const obj = yaml.load(data) as ISeqProfileConfig | null
  return {
    prepend: obj?.prepend || [],
    append: obj?.append || [],
    delete: normalizeDeleteSeq(obj?.delete),
  }
}

/**
 * Reorder array items
 */
export const reorderArray = <T>(
  list: T[],
  startIndex: number,
  endIndex: number,
): T[] => {
  const result = Array.from(list)
  const [removed] = result.splice(startIndex, 1)
  result.splice(endIndex, 0, removed)
  return result
}

/**
 * Validate group configuration
 */
export const validateGroupName = (name: string): boolean => {
  return name.trim().length > 0
}

/**
 * Check if group name exists in lists
 */
export const isGroupNameExists = (
  name: string,
  prependSeq: IProxyGroupConfig[],
  groupList: IProxyGroupConfig[],
  appendSeq: IProxyGroupConfig[],
): boolean => {
  return [...prependSeq, ...groupList, ...appendSeq].some(
    (item) => item.name === name,
  )
}
