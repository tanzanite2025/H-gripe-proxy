export function isPrimaryProfileItem(
  item: IProfileItem | null | undefined,
): item is IProfileItem {
  if (!item?.uid) {
    return false
  }

  if (item.type === 'remote' || item.type === 'local') {
    return true
  }

  const uidPrefix = item.uid.charAt(0)

  if (uidPrefix === 'R') {
    return !!item.url
  }

  if (uidPrefix === 'L') {
    return !item.url
  }

  return false
}

export function collectPrimaryProfileItems(
  items: Array<IProfileItem | null | undefined> | undefined,
) {
  return (items ?? []).filter(isPrimaryProfileItem)
}

export function isRemotePrimaryProfileItem(
  item: IProfileItem | null | undefined,
): item is IProfileItem {
  if (!isPrimaryProfileItem(item)) {
    return false
  }

  if (item.type === 'remote') {
    return true
  }

  return item.uid.startsWith('R') && !!item.url
}

export function isLocalPrimaryProfileItem(
  item: IProfileItem | null | undefined,
): item is IProfileItem {
  if (!isPrimaryProfileItem(item)) {
    return false
  }

  if (item.type === 'local') {
    return true
  }

  return item.uid.startsWith('L') && !item.url
}
