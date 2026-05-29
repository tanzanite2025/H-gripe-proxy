import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { UnlistenFn } from '@tauri-apps/api/event'
import { useLockFn } from 'ahooks'
import { Globe } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'


import { BaseLoading } from '@/components/base'
import { Divider, Menu, MenuItem } from '@/components/tailwind'
import { useIconCache, useListen } from '@/hooks/system'
import { cmdTestDelay } from '@/services/cmds'
import delayManager from '@/services/delay'
import { showNotice } from '@/services/notice-service'
import { cn } from '@/utils/cn'
import { debugLog } from '@/utils/misc'

import { TestBox } from './test-box'

interface Props {
  id: string
  itemData: IVergeTestItem
  onEdit: () => void
  onDelete: (uid: string) => void
}

export const TestItem = ({
  id,
  itemData,
  onEdit,
  onDelete: removeTest,
}: Props) => {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({
    id,
  })

  const { t } = useTranslation()
  const [anchorEl, setAnchorEl] = useState<any>(null)
  const [position, setPosition] = useState({ left: 0, top: 0 })
  const [delay, setDelay] = useState(-1)
  const { uid, name, icon, url } = itemData
  const iconCachePath = useIconCache({ icon, cacheKey: uid })
  const { addListener } = useListen()

  const onDelay = useCallback(async () => {
    setDelay(-2)
    const result = await cmdTestDelay(url)
    setDelay(result)
  }, [url])

  const onEditTest = () => {
    setAnchorEl(null)
    onEdit()
  }

  const onDelete = useLockFn(async () => {
    setAnchorEl(null)
    try {
      removeTest(uid)
    } catch (err: any) {
      showNotice.error(err)
    }
  })

  const menu = [
    { label: 'Edit', handler: onEditTest },
    { label: 'Delete', handler: onDelete },
  ]

  useEffect(() => {
    let unlistenFn: UnlistenFn | null = null

    const setupListener = async () => {
      if (unlistenFn) {
        unlistenFn()
      }
      unlistenFn = await addListener('verge://test-all', () => {
        onDelay()
      })
    }

    setupListener()

    return () => {
      if (unlistenFn) {
        debugLog(
          `TestItem for ${id} unmounting or url changed, cleaning up test-all listener.`,
        )
        unlistenFn()
      }
    }
  }, [url, addListener, onDelay, id])

  return (
    <div
      className="relative"
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
        zIndex: isDragging ? 9999 : undefined,
      }}
    >
      <TestBox
        onContextMenu={(event) => {
          const { clientX, clientY } = event
          setPosition({ top: clientY, left: clientX })
          setAnchorEl(event.currentTarget)
          event.preventDefault()
        }}
      >
        <div
          className="relative cursor-move"
          ref={setNodeRef}
          {...attributes}
          {...listeners}
        >
          {icon && icon.trim() !== '' ? (
            <div className="flex justify-center">
              {icon.trim().startsWith('http') && (
                <img
                  src={iconCachePath === '' ? icon : iconCachePath}
                  className="h-10 w-10 object-contain"
                  alt={name}
                />
              )}
              {icon.trim().startsWith('data') && (
                <img 
                  src={icon} 
                  className="h-10 w-10 object-contain"
                  alt={name}
                />
              )}
              {icon.trim().startsWith('<svg') && (
                <img
                  src={`data:image/svg+xml;base64,${btoa(icon)}`}
                  className="h-10 w-10 object-contain"
                  alt={name}
                />
              )}
            </div>
          ) : (
            <div className="flex justify-center">
              <Globe className="h-10 w-10" />
            </div>
          )}

          <div className="flex justify-center">{name}</div>
        </div>
        <Divider className="mt-2" />
        <div className="mt-2 flex justify-center text-primary dark:text-primary-dark-mode">
          {delay === -2 && (
            <div className="rounded-lg px-2 py-1 text-xs">
              <BaseLoading />
            </div>
          )}

          {delay === -1 && (
            <div
              className={cn(
                'the-check rounded-lg px-2 py-1 text-xs',
                'hover:bg-primary/15 dark:hover:bg-primary-dark-mode/15'
              )}
              onClick={(e) => {
                e.preventDefault()
                e.stopPropagation()
                onDelay()
              }}
            >
              {t('tests.components.item.actions.test')}
            </div>
          )}

          {delay >= 0 && (
            <div
              className={cn(
                'the-delay rounded-lg px-2 py-1 text-xs',
                'hover:bg-primary/15 dark:hover:bg-primary-dark-mode/15'
              )}
              style={{ color: delayManager.formatDelayColor(delay) }}
              onClick={(e) => {
                e.preventDefault()
                e.stopPropagation()
                onDelay()
              }}
            >
              {delayManager.formatDelay(delay)}
            </div>
          )}
        </div>
      </TestBox>

      <Menu
        open={!!anchorEl}
        anchorEl={anchorEl}
        onClose={() => setAnchorEl(null)}
        anchorPosition={position}
        anchorReference="anchorPosition"
        onContextMenu={(e) => {
          setAnchorEl(null)
          e.preventDefault()
        }}
      >
        {menu.map((item) => (
          <MenuItem
            key={item.label}
            onClick={item.handler}
          >
            {t(item.label)}
          </MenuItem>
        ))}
      </Menu>
    </div>
  )
}
