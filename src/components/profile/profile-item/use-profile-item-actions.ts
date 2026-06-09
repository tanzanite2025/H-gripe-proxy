import { useLockFn } from 'ahooks'
import { useCallback, useMemo, useState } from 'react'

import { openWebUrl, updateProfile, viewProfile } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

import {
  profileItemMenuLabels,
  type ContextMenuItem,
} from './shared'
import type { ProfileItemDialogsController } from './use-profile-item-dialogs'

type ProfileUpdateMode = 'direct' | 'proxy'

interface UseProfileItemActionsParams {
  itemData: IProfileItem
  dialogs: ProfileItemDialogsController
  mutateProfiles: () => Promise<void>
  setProfileLoading: (nextLoading: boolean) => void
  onEdit: () => void
  onSelect: (force: boolean) => void | Promise<void>
  batchMode?: boolean
  onSelectionChange?: () => void
}

function buildUpdateOption(
  mode: ProfileUpdateMode,
  option?: IProfileOption,
): Partial<IProfileOption> {
  if (mode === 'direct') {
    return {
      with_proxy: false,
      self_proxy: false,
    }
  }

  if (option?.self_proxy) {
    return {
      with_proxy: false,
      self_proxy: true,
    }
  }

  return {
    with_proxy: true,
    self_proxy: false,
  }
}

export function useProfileItemActions({
  itemData,
  dialogs,
  mutateProfiles,
  setProfileLoading,
  onEdit,
  onSelect,
  batchMode,
  onSelectionChange,
}: UseProfileItemActionsParams) {
  const [menuOpen, setMenuOpen] = useState(false)
  const [position, setPosition] = useState({ left: 0, top: 0 })

  const { uid, option } = itemData
  const hasUrl = !!itemData.url
  const hasHome = !!itemData.home

  const closeMenu = useCallback(() => {
    setMenuOpen(false)
  }, [])

  const openMenu = useCallback((event: React.MouseEvent) => {
    const { clientX, clientY } = event
    setPosition({ top: clientY, left: clientX })
    setMenuOpen(true)
    event.preventDefault()
  }, [])

  const handleOpenHome = useCallback(() => {
    closeMenu()
    void openWebUrl(itemData.home ?? '')
  }, [closeMenu, itemData.home])

  const handleEditInfo = useCallback(() => {
    closeMenu()
    onEdit()
  }, [closeMenu, onEdit])

  const handleShareQrCode = useCallback(() => {
    closeMenu()
    dialogs.openQr()
  }, [closeMenu, dialogs])

  const handleEditFile = useCallback(() => {
    closeMenu()
    dialogs.openFile()
  }, [closeMenu, dialogs])

  const handleEditRules = useCallback(() => {
    closeMenu()
    dialogs.openRules()
  }, [closeMenu, dialogs])

  const handleEditProxies = useCallback(() => {
    closeMenu()
    dialogs.openProxies()
  }, [closeMenu, dialogs])

  const handleEditScript = useCallback(() => {
    closeMenu()
    dialogs.openScript()
  }, [closeMenu, dialogs])

  const handleForceSelect = useCallback(() => {
    closeMenu()
    void onSelect(true)
  }, [closeMenu, onSelect])

  const handleDeleteRequest = useCallback(() => {
    closeMenu()

    if (batchMode) {
      onSelectionChange?.()
      return
    }

    dialogs.openConfirm()
  }, [batchMode, closeMenu, dialogs, onSelectionChange])

  const handleOpenFileOnDisk = useLockFn(async () => {
    closeMenu()

    try {
      await viewProfile(uid)
    } catch (error) {
      showNotice.error(error)
    }
  })

  const handleUpdate = useLockFn(async (mode: ProfileUpdateMode) => {
    setProfileLoading(true)

    try {
      await updateProfile(uid, buildUpdateOption(mode, option))
      void mutateProfiles()
    } catch (error) {
      showNotice.error(error)
    } finally {
      setProfileLoading(false)
    }
  })

  const menuItems = useMemo<ContextMenuItem[]>(() => {
    const commonItems: ContextMenuItem[] = [
      {
        label: profileItemMenuLabels.editInfo,
        handler: handleEditInfo,
        disabled: false,
      },
      {
        label: profileItemMenuLabels.editFile,
        handler: handleEditFile,
        disabled: false,
      },
      {
        label: profileItemMenuLabels.editRules,
        handler: handleEditRules,
        disabled: !option?.rules,
      },
      {
        label: profileItemMenuLabels.extendScript,
        handler: handleEditScript,
        disabled: !option?.script,
      },
      {
        label: profileItemMenuLabels.openFile,
        handler: () => {
          void handleOpenFileOnDisk()
        },
        disabled: false,
      },
      {
        label: profileItemMenuLabels.delete,
        handler: handleDeleteRequest,
        disabled: false,
        destructive: true,
      },
    ]

    if (!hasUrl) {
      return commonItems
    }

    return [
      ...(hasHome
        ? [
            {
              label: profileItemMenuLabels.home,
              handler: handleOpenHome,
              disabled: false,
            } satisfies ContextMenuItem,
          ]
        : []),
      {
        label: profileItemMenuLabels.shareQrCode,
        handler: handleShareQrCode,
        disabled: false,
      },
      ...commonItems,
    ]
  }, [
    handleDeleteRequest,
    handleEditFile,
    handleEditInfo,
    handleEditRules,
    handleEditScript,
    handleOpenFileOnDisk,
    handleOpenHome,
    handleShareQrCode,
    hasHome,
    hasUrl,
    option?.rules,
    option?.script,
  ])

  return {
    menuOpen,
    position,
    menuItems,
    closeMenu,
    openMenu,
    handleShareQrCode,
    handleEditProxies,
    handleForceSelect,
    handleDirectUpdate: () => {
      void handleUpdate('direct')
    },
    handleProxyUpdate: () => {
      void handleUpdate('proxy')
    },
  }
}
