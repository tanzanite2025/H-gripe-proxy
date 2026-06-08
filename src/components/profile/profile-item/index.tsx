import { useSortable } from '@dnd-kit/sortable'
import { useLockFn } from 'ahooks'
import { useEffect, useReducer, useState } from 'react'

import {
  openWebUrl,
  updateProfile,
  viewProfile,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { useLoadingCache, useSetLoadingCache } from '@/services/states'

import { ProfileItemUI } from '../profile-item-ui'
import { ProfileItemDialogs } from './profile-item-dialogs'
import { ProfileItemMenu } from './profile-item-menu'
import {
  formatExpireDate,
  parseProfileUrl,
  profileItemMenuLabels,
  type ContextMenuItem,
  type ProfileItemProps,
} from './shared'
import { useNextUpdateDisplay } from './use-next-update-display'
import { useProfileItemDialogs } from './use-profile-item-dialogs'

export const ProfileItem = ({
  id,
  selected,
  activating,
  itemData,
  mutateProfiles,
  onSelect,
  onEdit,
  onSave,
  onDelete,
  batchMode,
  isSelected,
  onSelectionChange,
}: ProfileItemProps) => {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id })
  const [menuOpen, setMenuOpen] = useState(false)
  const [position, setPosition] = useState({ left: 0, top: 0 })
  const loadingCache = useLoadingCache()
  const setLoadingCache = useSetLoadingCache()

  const { uid, name = 'Profile', extra, updated = 0, option } = itemData

  const {
    showNextUpdate,
    nextUpdateTime,
    toggleUpdateTimeDisplay,
    refreshNextUpdateTime,
  } = useNextUpdateDisplay({
    uid,
    updateInterval: itemData.option?.update_interval,
    updated,
  })
  const dialogs = useProfileItemDialogs({
    uid,
    option,
    onSave,
  })

  const hasUrl = !!itemData.url
  const hasExtra = !!extra
  const hasHome = !!itemData.home
  const { upload = 0, download = 0, total = 0 } = extra ?? {}
  const from = parseProfileUrl(itemData.url)
  const description = itemData.desc
  const expire = formatExpireDate(extra?.expire)
  const progress = Math.min(
    Math.round(((download + upload) * 100) / (total + 0.01)) + 1,
    100,
  )
  const loading = loadingCache[uid] ?? false

  const [, forceRefresh] = useReducer((value: number) => value + 1, 0)

  useEffect(() => {
    if (!hasUrl) return

    let timer: ReturnType<typeof setTimeout> | undefined

    const scheduleRefresh = () => {
      const now = Date.now()
      const lastUpdate = updated * 1000

      if (now - lastUpdate >= 24 * 36e5) return

      const wait = now - lastUpdate >= 36e5 ? 30e5 : 5e4

      timer = setTimeout(() => {
        forceRefresh()
        scheduleRefresh()
      }, wait)
    }

    scheduleRefresh()

    return () => {
      if (timer) {
        clearTimeout(timer)
        timer = undefined
      }
    }
  }, [forceRefresh, hasUrl, updated])

  const closeMenu = () => {
    setMenuOpen(false)
  }

  const onOpenHome = () => {
    closeMenu()
    void openWebUrl(itemData.home ?? '')
  }

  const onEditInfo = () => {
    closeMenu()
    onEdit()
  }

  const onShareQrCode = () => {
    closeMenu()
    dialogs.openQr()
  }

  const onEditFile = () => {
    closeMenu()
    dialogs.openFile()
  }

  const onEditRules = () => {
    closeMenu()
    dialogs.openRules()
  }

  const onEditProxies = () => {
    closeMenu()
    dialogs.openProxies()
  }

  const onEditGroups = () => {
    closeMenu()
    dialogs.openGroups()
  }

  const onEditMerge = () => {
    closeMenu()
    dialogs.openMerge()
  }

  const onEditScript = () => {
    closeMenu()
    dialogs.openScript()
  }

  const onForceSelect = () => {
    closeMenu()
    void onSelect(true)
  }

  const handleDeleteRequest = () => {
    closeMenu()
    if (batchMode) {
      onSelectionChange?.()
      return
    }
    dialogs.openConfirm()
  }

  const onOpenFile = useLockFn(async () => {
    closeMenu()
    try {
      await viewProfile(uid)
    } catch (error) {
      showNotice.error(error)
    }
  })

  const onUpdate = useLockFn(async (type: 0 | 1 | 2): Promise<void> => {
    setLoadingCache((cache) => ({ ...cache, [uid]: true }))

    const nextOption: Partial<IProfileOption> = {}
    if (type === 0) {
      nextOption.with_proxy = false
      nextOption.self_proxy = false
    } else if (type === 2) {
      if (itemData.option?.self_proxy) {
        nextOption.with_proxy = false
        nextOption.self_proxy = true
      } else {
        nextOption.with_proxy = true
        nextOption.self_proxy = false
      }
    }

    try {
      const payload =
        Object.keys(nextOption).length > 0 ? nextOption : undefined
      await updateProfile(uid, payload)
      void mutateProfiles()
    } finally {
      setLoadingCache((cache) => ({ ...cache, [uid]: false }))
    }
  })

  const urlModeMenu: ContextMenuItem[] = [
    ...(hasHome
      ? [
          {
            label: profileItemMenuLabels.home,
            handler: onOpenHome,
            disabled: false,
          } satisfies ContextMenuItem,
        ]
      : []),
    {
      label: profileItemMenuLabels.shareQrCode,
      handler: onShareQrCode,
      disabled: false,
    },
    {
      label: profileItemMenuLabels.editInfo,
      handler: onEditInfo,
      disabled: false,
    },
    {
      label: profileItemMenuLabels.editFile,
      handler: onEditFile,
      disabled: false,
    },
    {
      label: profileItemMenuLabels.editRules,
      handler: onEditRules,
      disabled: !option?.rules,
    },
    {
      label: profileItemMenuLabels.extendConfig,
      handler: onEditMerge,
      disabled: !option?.merge,
    },
    {
      label: profileItemMenuLabels.extendScript,
      handler: onEditScript,
      disabled: !option?.script,
    },
    {
      label: profileItemMenuLabels.openFile,
      handler: () => {
        void onOpenFile()
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

  const fileModeMenu: ContextMenuItem[] = [
    {
      label: profileItemMenuLabels.editInfo,
      handler: onEditInfo,
      disabled: false,
    },
    {
      label: profileItemMenuLabels.editFile,
      handler: onEditFile,
      disabled: false,
    },
    {
      label: profileItemMenuLabels.editRules,
      handler: onEditRules,
      disabled: !option?.rules,
    },
    {
      label: profileItemMenuLabels.extendConfig,
      handler: onEditMerge,
      disabled: !option?.merge,
    },
    {
      label: profileItemMenuLabels.extendScript,
      handler: onEditScript,
      disabled: !option?.script,
    },
    {
      label: profileItemMenuLabels.openFile,
      handler: () => {
        void onOpenFile()
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

  useEffect(() => {
    const handleUpdateStarted = (event: Event) => {
      const customEvent = event as CustomEvent<{ uid?: string }>
      if (customEvent.detail?.uid === uid) {
        setLoadingCache((cache) => ({ ...cache, [uid]: true }))
      }
    }

    const handleUpdateCompleted = (event: Event) => {
      const customEvent = event as CustomEvent<{ uid?: string }>
      if (customEvent.detail?.uid === uid) {
        setLoadingCache((cache) => ({ ...cache, [uid]: false }))
        void mutateProfiles()
        if (showNextUpdate) {
          void refreshNextUpdateTime()
        }
      }
    }

    window.addEventListener('profile-update-started', handleUpdateStarted)
    window.addEventListener('profile-update-completed', handleUpdateCompleted)

    return () => {
      window.removeEventListener('profile-update-started', handleUpdateStarted)
      window.removeEventListener(
        'profile-update-completed',
        handleUpdateCompleted,
      )
    }
  }, [
    mutateProfiles,
    refreshNextUpdateTime,
    setLoadingCache,
    showNextUpdate,
    uid,
  ])

  return (
    <>
      <ProfileItemUI
        name={name}
        description={description}
        from={from}
        hasUrl={hasUrl}
        hasExtra={hasExtra}
        hasHome={hasHome}
        selected={selected}
        activating={activating}
        loading={loading}
        isDragging={isDragging}
        batchMode={batchMode}
        isSelected={isSelected}
        updated={updated}
        showNextUpdate={showNextUpdate}
        nextUpdateTime={nextUpdateTime}
        upload={upload}
        download={download}
        total={total}
        expire={expire}
        progress={progress}
        dragHandleProps={{
          ref: setNodeRef,
          attributes,
          listeners,
        }}
        transform={transform}
        transition={transition}
        onClick={(event) => {
          if (activating) {
            event.preventDefault()
            event.stopPropagation()
            return
          }
          void onSelect(false)
        }}
        onContextMenu={(event) => {
          const { clientX, clientY } = event
          setPosition({ top: clientY, left: clientX })
          setMenuOpen(true)
          event.preventDefault()
        }}
        onUseClick={() => {
          if (activating) {
            return
          }
          onForceSelect()
        }}
        onDirectUpdateClick={() => {
          if (activating || loading) {
            return
          }
          void onUpdate(0)
        }}
        onProxyUpdateClick={() => {
          if (activating || loading) {
            return
          }
          void onUpdate(2)
        }}
        onEditProxiesClick={() => {
          if (!option?.proxies) {
            return
          }
          onEditProxies()
        }}
        onEditGroupsClick={() => {
          if (!option?.groups) {
            return
          }
          onEditGroups()
        }}
        onShareQrCodeClick={() => {
          if (!hasUrl) {
            return
          }
          onShareQrCode()
        }}
        canEditProxies={!!option?.proxies}
        canEditGroups={!!option?.groups}
        onToggleUpdateTimeDisplay={toggleUpdateTimeDisplay}
        onSelectionChange={onSelectionChange}
      />

      <ProfileItemMenu
        open={menuOpen}
        position={position}
        items={hasUrl ? urlModeMenu : fileModeMenu}
        onClose={closeMenu}
      />

      <ProfileItemDialogs
        itemData={itemData}
        name={name}
        option={option}
        onSave={onSave}
        onDelete={onDelete}
        dialogs={dialogs}
      />
    </>
  )
}
