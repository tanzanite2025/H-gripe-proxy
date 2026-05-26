import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
} from '@dnd-kit/core'
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import {
  Box,
  List,
  Menu,
  MenuItem,
  Paper,
  SvgIcon,
  ThemeProvider,
} from '@mui/material'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import type { CSSProperties } from 'react'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Outlet, useLocation, useNavigate } from 'react-router'

import AppIcon from '@/assets/image/icon_dark.svg?react'
import { BaseErrorBoundary } from '@/components/base'
import { LayoutItem } from '@/components/layout/layout-item'
import { LayoutTraffic } from '@/components/layout/layout-traffic'
import { NoticeManager } from '@/components/layout/notice-manager'
import { UpdateButton } from '@/components/layout/update-button'
import { WindowControls } from '@/components/layout/window-controller'
import { useI18n, useWindowDecorations } from '@/hooks/ui'
import { useVerge } from '@/hooks/system'
import { useThemeMode } from '@/services/states'
import getSystem from '@/utils/misc'

import {
  useCustomTheme,
  useLayoutEvents,
  useLoadingOverlay,
  useNavMenuOrder,
} from './_layout/hooks'
import { handleNoticeMessage } from './_layout/utils'
import { navItems } from './_routers'
import LogsPage from './logs'

import 'dayjs/locale/ru'
import 'dayjs/locale/zh-cn'

export const portableFlag = false

type NavItem = (typeof navItems)[number]

type MenuContextPosition = { top: number; left: number }

interface SortableNavMenuItemProps {
  item: NavItem
  label: string
}

const SortableNavMenuItem = ({ item, label }: SortableNavMenuItemProps) => {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({
    id: item.path,
  })

  const style: CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
  }

  if (isDragging) {
    style.zIndex = 100
  }

  return (
    <LayoutItem
      to={item.path}
      icon={item.icon}
      sortable={{
        setNodeRef,
        attributes,
        listeners,
        style,
        isDragging,
      }}
    >
      {label}
    </LayoutItem>
  )
}

dayjs.extend(relativeTime)

const OS = getSystem()

const Layout = () => {
  const mode = useThemeMode()
  const { t } = useTranslation()
  const { theme } = useCustomTheme()
  const { verge, mutateVerge, patchVerge } = useVerge()
  const { language } = verge ?? {}
  const { switchLanguage } = useI18n()
  const navigate = useNavigate()
  const { pathname } = useLocation()
  const isLogsPage = pathname === '/logs'
  const logsPageMountedRef = useRef(false)
  if (isLogsPage) logsPageMountedRef.current = true
  const themeReady = useMemo(() => Boolean(theme), [theme])

  const [menuUnlocked, setMenuUnlocked] = useState(false)
  const [menuContextPosition, setMenuContextPosition] =
    useState<MenuContextPosition | null>(null)

  const windowControlsRef = useRef<any>(null)
  const { decorated } = useWindowDecorations()

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 6,
      },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  )

  const handleMenuOrderOptimisticUpdate = useCallback(
    (order: string[]) => {
      mutateVerge(
        (prev) => (prev ? { ...prev, menu_order: order } : prev),
        false,
      )
    },
    [mutateVerge],
  )

  const handleMenuOrderPersist = useCallback(
    (order: string[]) => patchVerge({ menu_order: order }),
    [patchVerge],
  )

  const {
    menuOrder,
    navItemMap,
    handleMenuDragEnd,
    isDefaultOrder,
    resetMenuOrder,
  } = useNavMenuOrder({
    enabled: menuUnlocked,
    items: navItems,
    storedOrder: verge?.menu_order,
    onOptimisticUpdate: handleMenuOrderOptimisticUpdate,
    onPersist: handleMenuOrderPersist,
  })

  const handleMenuContextMenu = useCallback(
    (event: React.MouseEvent<HTMLElement>) => {
      event.preventDefault()
      event.stopPropagation()
      setMenuContextPosition({ top: event.clientY, left: event.clientX })
    },
    [],
  )

  const handleMenuContextClose = useCallback(() => {
    setMenuContextPosition(null)
  }, [])

  const handleResetMenuOrder = useCallback(() => {
    setMenuContextPosition(null)
    void resetMenuOrder()
  }, [resetMenuOrder])

  const handleUnlockMenu = useCallback(() => {
    setMenuUnlocked(true)
    setMenuContextPosition(null)
  }, [])

  const handleLockMenu = useCallback(() => {
    setMenuUnlocked(false)
    setMenuContextPosition(null)
  }, [])

  useLoadingOverlay(themeReady)

  const handleNotice = useCallback(
    (payload: [string, string]) => {
      const [status, msg] = payload
      try {
        handleNoticeMessage(status, msg, t, navigate)
      } catch (error) {
        console.error('[通知处理] 失败:', error)
      }
    },
    [t, navigate],
  )

  useLayoutEvents(handleNotice)

  useEffect(() => {
    if (language) {
      dayjs.locale(language === 'zh' ? 'zh-cn' : language)
      switchLanguage(language)
    }
  }, [language, switchLanguage])

  if (!themeReady) {
    return (
      <div
        style={{
          width: '100vw',
          height: '100vh',
          background: mode === 'light' ? '#fff' : '#181a1b',
          transition: 'background 0.2s',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: mode === 'light' ? '#333' : '#fff',
        }}
      ></div>
    )
  }

  return (
    <ThemeProvider theme={theme}>
      {/* 左侧底部窗口控制按钮 */}
      <NoticeManager position={verge?.notice_position} />
      <div
        style={{
          animation: 'fadeIn 0.5s',
          WebkitAnimation: 'fadeIn 0.5s',
        }}
      />
      <style>
        {`
            @keyframes fadeIn {
              from { opacity: 0; }
              to { opacity: 1; }
            }
          `}
      </style>
      <Paper
        square
        elevation={0}
        className={`${OS} layout`}
        style={{
          borderTopLeftRadius: '0px',
          borderTopRightRadius: '0px',
        }}
        onContextMenu={(e) => {
          if (
            OS === 'windows' &&
            !['input', 'textarea'].includes(
              e.currentTarget.tagName.toLowerCase(),
            ) &&
            !e.currentTarget.isContentEditable
          ) {
            e.preventDefault()
          }
        }}
        sx={[
          ({ palette }) => ({ bgcolor: palette.background.paper }),
          OS === 'linux'
            ? {
                borderRadius: '8px',
                width: '100vw',
                height: '100vh',
              }
            : {},
        ]}
      >
        {/* 顶部贯穿式页眉与导航控制台 */}
        <div className="layout-header">
          <div className="layout-header__drag-zone" data-tauri-drag-region="true" />
          {/* 左侧 Logo */}
          <div className="the-logo">
            <SvgIcon
              component={AppIcon}
              style={{
                height: '28px',
                width: '28px',
              }}
              inheritViewBox
            />
          </div>

          {/* 中间 Tab 导航 */}
          <div className="the-menu-wrapper">
            {menuUnlocked ? (
              <DndContext
                sensors={sensors}
                collisionDetection={closestCenter}
                onDragEnd={handleMenuDragEnd}
              >
                <SortableContext items={menuOrder}>
                  <List
                    className="the-menu"
                    onContextMenu={handleMenuContextMenu}
                    style={{ display: 'flex', flexDirection: 'row', padding: 0 }}
                  >
                    {menuOrder.map((path) => {
                      const item = navItemMap.get(path)
                      if (!item) {
                        return null
                      }
                      return (
                        <SortableNavMenuItem
                          key={item.path}
                          item={item}
                          label={t(item.label)}
                        />
                      )
                    })}
                  </List>
                </SortableContext>
              </DndContext>
            ) : (
              <List
                className="the-menu"
                onContextMenu={handleMenuContextMenu}
                style={{ display: 'flex', flexDirection: 'row', padding: 0 }}
              >
                {menuOrder.map((path) => {
                  const item = navItemMap.get(path)
                  if (!item) {
                    return null
                  }
                  return (
                    <LayoutItem key={item.path} to={item.path} icon={item.icon}>
                      {t(item.label)}
                    </LayoutItem>
                  )
                })}
              </List>
            )}
          </div>

          {/* 右侧：状态组件、升级按钮及窗口控件 */}
          <div className="layout-header__right">
            <div className="the-traffic">
              <LayoutTraffic horizontal />
            </div>

            <UpdateButton className="the-newbtn" />

            {!decorated && OS !== 'macos' && (
              <div className="the-window-ctrls">
                <WindowControls ref={windowControlsRef} />
              </div>
            )}
          </div>
        </div>

        {/* 导航重新排序时的警示条 */}
        {menuUnlocked && (
          <Box
            sx={(theme) => ({
              px: 1.5,
              py: 0.5,
              width: '100%',
              fontSize: 11,
              fontWeight: 600,
              textAlign: 'center',
              color: theme.palette.warning.contrastText,
              bgcolor:
                theme.palette.mode === 'light'
                  ? theme.palette.warning.main
                  : theme.palette.warning.dark,
              zIndex: 10,
            })}
          >
            {t('layout.components.navigation.menu.reorderMode')}
          </Box>
        )}

        <Menu
          open={Boolean(menuContextPosition)}
          onClose={handleMenuContextClose}
          anchorReference="anchorPosition"
          anchorPosition={
            menuContextPosition
              ? {
                  top: menuContextPosition.top,
                  left: menuContextPosition.left,
                }
              : undefined
          }
          transitionDuration={200}
          slotProps={{
            list: {
              sx: { py: 0.5 },
            },
          }}
        >
          <MenuItem
            onClick={menuUnlocked ? handleLockMenu : handleUnlockMenu}
            dense
          >
            {menuUnlocked
              ? t('layout.components.navigation.menu.lock')
              : t('layout.components.navigation.menu.unlock')}
          </MenuItem>
          <MenuItem
            onClick={handleResetMenuOrder}
            dense
            disabled={isDefaultOrder}
          >
            {t('layout.components.navigation.menu.restoreDefaultOrder')}
          </MenuItem>
        </Menu>

        {/* 主内容渲染区域 */}
        <div className="layout-body">
          <BaseErrorBoundary>
            <Outlet />
          </BaseErrorBoundary>
          {logsPageMountedRef.current && (
            <div
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                right: 0,
                bottom: 0,
                display: isLogsPage ? undefined : 'none',
              }}
            >
              <LogsPage />
            </div>
          )}
        </div>
      </Paper>
    </ThemeProvider>
  )
}

export default Layout