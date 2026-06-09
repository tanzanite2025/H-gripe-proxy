import dayjs from 'dayjs'

import type { TranslationKey } from '@/types/generated/i18n-keys'

export interface ProfileItemProps {
  id: string
  selected: boolean
  activating: boolean
  itemData: IProfileItem
  mutateProfiles: () => Promise<void>
  onSelect: (force: boolean) => void | Promise<void>
  onEdit: () => void
  onSave?: (prev?: string, curr?: string) => void | Promise<void>
  onDelete: () => void | Promise<void>
  batchMode?: boolean
  isSelected?: boolean
  onSelectionChange?: () => void
}

export interface ContextMenuItem {
  label: TranslationKey
  handler: () => void
  disabled: boolean
  destructive?: boolean
}

export const profileItemMenuLabels = {
  home: 'profiles.components.menu.home',
  select: 'profiles.components.menu.select',
  shareQrCode: 'profiles.components.menu.shareQrCode',
  editInfo: 'profiles.components.menu.editInfo',
  editFile: 'profiles.components.menu.editFile',
  editRules: 'profiles.components.menu.editRules',
  editProxies: 'profiles.components.menu.editProxies',
  extendScript: 'profiles.components.menu.extendScript',
  openFile: 'profiles.components.menu.openFile',
  update: 'profiles.components.menu.update',
  updateViaProxy: 'profiles.components.menu.updateViaProxy',
  delete: 'shared.actions.delete',
} as const satisfies Record<string, TranslationKey>

export function parseProfileUrl(url?: string) {
  if (!url) return ''
  const regex = /https?:\/\/(.+?)\//
  const result = url.match(regex)
  return result ? result[1] : 'local file'
}

export function formatExpireDate(expire?: number) {
  if (!expire) return '-'
  return dayjs(expire * 1000).format('YYYY-MM-DD')
}

export function buildProfileQrCodeValue(url: string, name: string) {
  const separator = url.includes('?') ? '&' : '?'
  return `${url}${separator}name=${encodeURIComponent(name)}`
}
