import { invoke } from '@tauri-apps/api/core'
import { useLockFn } from 'ahooks'
import { Clock, XCircle, CheckCircle, HelpCircle, RefreshCw } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseEmpty, BasePage } from '@/components/base'
import {
  Box,
  Button,
  Card,
  Chip,
  CircularProgress,
  Divider,
  Grid,
  IconButton,
  Tooltip,
  Typography,
} from '@/components/tailwind'
import { showNotice } from '@/services/notice-service'

interface UnlockItem {
  name: string
  status: string
  region?: string | null
  check_time?: string | null
}

const UNLOCK_RESULTS_STORAGE_KEY = 'clash_verge_unlock_results'
const UNLOCK_RESULTS_TIME_KEY = 'clash_verge_unlock_time'

const STATUS_LABEL_KEYS: Record<string, string> = {
  Pending: 'tests.statuses.test.pending',
  Yes: 'tests.statuses.test.yes',
  No: 'tests.statuses.test.no',
  Failed: 'tests.statuses.test.failed',
  Completed: 'tests.statuses.test.completed',
  'Disallowed ISP': 'tests.statuses.test.disallowedIsp',
  'Originals Only': 'tests.statuses.test.originalsOnly',
  'No (IP Banned By Disney+)': 'tests.statuses.test.noDisney',
  'Unsupported Country/Region': 'tests.statuses.test.unsupportedRegion',
  'Failed (Network Connection)': 'tests.statuses.test.failedNetwork',
}

const normalizeUnlockName = (name: string) => name.trim().toLowerCase()

const getStatusPriority = (status: string) => (status === 'Pending' ? 0 : 1)
const mergeOptionalFields = (preferred: UnlockItem, fallback: UnlockItem) => ({
  ...preferred,
  region: preferred.region ?? fallback.region,
  check_time: preferred.check_time ?? fallback.check_time,
})

const dedupeUnlockItems = (items: UnlockItem[]) => {
  const map = new Map<string, UnlockItem>()

  items.forEach((item) => {
    const key = normalizeUnlockName(item.name)
    const existing = map.get(key)

    if (!existing) {
      map.set(key, item)
      return
    }

    const existingPriority = getStatusPriority(existing.status)
    const itemPriority = getStatusPriority(item.status)

    if (itemPriority > existingPriority) {
      map.set(key, mergeOptionalFields(item, existing))
      return
    }

    if (itemPriority < existingPriority) {
      map.set(key, mergeOptionalFields(existing, item))
      return
    }

    map.set(key, mergeOptionalFields(item, existing))
  })

  return Array.from(map.values())
}

const UnlockPage = () => {
  const { t } = useTranslation()
  const [isDark, setIsDark] = useState(false)

  // 检测暗色模式
  useEffect(() => {
    const checkDarkMode = () => {
      const next = document.documentElement.classList.contains('dark')
      // defer state update to avoid synchronous set warning
      requestAnimationFrame(() => {
        setIsDark((prev) => (prev === next ? prev : next))
      })
    }
    checkDarkMode()
    
    // 监听暗色模式变化
    const observer = new MutationObserver(checkDarkMode)
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class'],
    })
    
    return () => observer.disconnect()
  }, [])

  const [unlockItems, setUnlockItems] = useState<UnlockItem[]>([])
  const [isCheckingAll, setIsCheckingAll] = useState(false)
  const [loadingItems, setLoadingItems] = useState<string[]>([])

  const sortItemsByName = useCallback((items: UnlockItem[]) => {
    return [...items].sort((a, b) => a.name.localeCompare(b.name))
  }, [])

  const mergeUnlockItems = useCallback(
    (defaults: UnlockItem[], existing?: UnlockItem[] | null) => {
      if (!existing || existing.length === 0) {
        return defaults
      }

      const normalizedExisting = dedupeUnlockItems(existing)
      const existingMap = new Map(
        normalizedExisting.map((item) => [
          normalizeUnlockName(item.name),
          item,
        ]),
      )
      const merged = defaults.map((item) => {
        const normalizedName = normalizeUnlockName(item.name)
        const matchedItem = existingMap.get(normalizedName)
        if (matchedItem) {
          return { ...matchedItem, name: item.name }
        }
        return item
      })

      const mergedNameSet = new Set(
        merged.map((item) => normalizeUnlockName(item.name)),
      )
      normalizedExisting.forEach((item) => {
        const normalizedName = normalizeUnlockName(item.name)
        if (!mergedNameSet.has(normalizedName)) {
          merged.push(item)
          mergedNameSet.add(normalizedName)
        }
      })

      return merged
    },
    [],
  )

  // 保存测试结果到本地存储
  const saveResultsToStorage = useCallback(
    (items: UnlockItem[], time: string | null) => {
      try {
        localStorage.setItem(UNLOCK_RESULTS_STORAGE_KEY, JSON.stringify(items))
        if (time) {
          localStorage.setItem(UNLOCK_RESULTS_TIME_KEY, time)
        }
      } catch (err) {
        console.error('Failed to save results to storage:', err)
      }
    },
    [],
  )

  const loadResultsFromStorage = useCallback((): {
    items: UnlockItem[] | null
    time: string | null
  } => {
    try {
      const itemsJson = localStorage.getItem(UNLOCK_RESULTS_STORAGE_KEY)
      const time = localStorage.getItem(UNLOCK_RESULTS_TIME_KEY)

      if (itemsJson) {
        const parsedItems = JSON.parse(itemsJson) as UnlockItem[]
        return {
          items: dedupeUnlockItems(parsedItems),
          time,
        }
      }
    } catch (err) {
      console.error('Failed to load results from storage:', err)
    }

    return { items: null, time: null }
  }, [])

  const getUnlockItems = useCallback(
    async (
      existingItems: UnlockItem[] | null = null,
      existingTime: string | null = null,
    ) => {
      try {
        const defaultItems = await invoke<UnlockItem[]>('get_unlock_items')
        const mergedItems = mergeUnlockItems(defaultItems, existingItems)
        const sortedItems = sortItemsByName(mergedItems)

        setUnlockItems(sortedItems)
        saveResultsToStorage(
          sortedItems,
          existingItems && existingItems.length > 0 ? existingTime : null,
        )
      } catch (err: any) {
        console.error('Failed to get unlock items:', err)
      }
    },
    [mergeUnlockItems, saveResultsToStorage, sortItemsByName],
  )

  useEffect(() => {
    void (async () => {
      const { items: storedItems, time: storedTime } = loadResultsFromStorage()

      if (storedItems && storedItems.length > 0) {
        setUnlockItems(sortItemsByName(storedItems))
        await getUnlockItems(storedItems, storedTime)
      } else {
        await getUnlockItems()
      }
    })()
  }, [getUnlockItems, loadResultsFromStorage, sortItemsByName])

  const invokeWithTimeout = async <T,>(
    cmd: string,
    args?: any,
    timeout = 15000,
  ): Promise<T> => {
    return Promise.race([
      invoke<T>(cmd, args),
      new Promise<T>((_, reject) =>
        setTimeout(
          () =>
            reject(new Error(t('tests.unlock.page.messages.detectionTimeout'))),
          timeout,
        ),
      ),
    ])
  }

  // 执行全部项目检测
  const checkAllMedia = useLockFn(async () => {
    try {
      setIsCheckingAll(true)
      const result = await invokeWithTimeout<UnlockItem[]>('check_media_unlock')
      const sortedItems = sortItemsByName(dedupeUnlockItems(result))

      setUnlockItems(sortedItems)
      const currentTime = new Date().toLocaleString()

      saveResultsToStorage(sortedItems, currentTime)

      setIsCheckingAll(false)
    } catch (err: any) {
      setIsCheckingAll(false)
      showNotice.error('tests.unlock.page.messages.detectionTimeout', err)
      console.error('Failed to check media unlock:', err)
    }
  })

  // 检测单个流媒体服务
  const checkSingleMedia = useLockFn(async (name: string) => {
    try {
      setLoadingItems((prev) => [...prev, name])
      const result = await invokeWithTimeout<UnlockItem[]>('check_media_unlock')
      const dedupedResult = dedupeUnlockItems(result)

      const normalizedTargetName = normalizeUnlockName(name)
      const targetItem = dedupedResult.find(
        (item: UnlockItem) =>
          normalizeUnlockName(item.name) === normalizedTargetName,
      )

      if (targetItem) {
        const updatedItems = sortItemsByName(
          dedupeUnlockItems(
            unlockItems.map((item: UnlockItem) =>
              normalizeUnlockName(item.name) === normalizedTargetName
                ? targetItem
                : item,
            ),
          ),
        )

        setUnlockItems(updatedItems)
        const currentTime = new Date().toLocaleString()

        saveResultsToStorage(updatedItems, currentTime)
      }

      setLoadingItems((prev) => prev.filter((item) => item !== name))
    } catch (err: any) {
      setLoadingItems((prev) => prev.filter((item) => item !== name))
      showNotice.error(
        'tests.unlock.page.messages.detectionFailedWithName',
        { name },
        err,
      )
      console.error(`Failed to check ${name}:`, err)
    }
  })

  // 状态颜色
  const getStatusColor = (status: string) => {
    if (status === 'Pending') return 'default'
    if (status === 'Yes') return 'success'
    if (status === 'No') return 'error'
    if (status === 'Soon') return 'warning'
    if (status.includes('Failed')) return 'error'
    if (status === 'Completed') return 'info'
    if (
      status === 'Disallowed ISP' ||
      status === 'Blocked' ||
      status === 'Unsupported Country/Region'
    ) {
      return 'error'
    }
    return 'default'
  }

  // 状态图标
  const getStatusIcon = (status: string) => {
    const iconProps = { className: 'h-4 w-4' }
    if (status === 'Pending') return <Clock {...iconProps} />
    if (status === 'Yes') return <CheckCircle {...iconProps} />
    if (status === 'No') return <XCircle {...iconProps} />
    if (status === 'Soon') return <Clock {...iconProps} />
    if (status.includes('Failed')) return <HelpCircle {...iconProps} />
    return <HelpCircle {...iconProps} />
  }

  // 边框色
  const getStatusBorderColor = (status: string) => {
    if (status === 'Yes') return '#10b981' // green-500
    if (status === 'No') return '#ef4444' // red-500
    if (status === 'Soon') return '#f59e0b' // amber-500
    if (status.includes('Failed')) return '#ef4444' // red-500
    if (status === 'Completed') return '#3b82f6' // blue-500
    return '#374151' // gray-700 : gray-200
  }

  return (
    <BasePage
      title={t('layout.components.navigation.tabs.unlock')}
      header={
        <Box className="flex items-center gap-1">
          <Button
            variant="primary"
            size="small"
            disabled={isCheckingAll}
            onClick={checkAllMedia}
            startIcon={
              isCheckingAll ? (
                <CircularProgress size={16} color="inherit" />
              ) : (
                <RefreshCw className="h-5 w-5" />
              )
            }
          >
            {isCheckingAll
              ? t('tests.unlock.page.actions.testing')
              : t('tests.page.actions.testAll')}
          </Button>
        </Box>
      }
    >
      {unlockItems.length === 0 ? (
        <Box className="flex justify-center items-center h-1/2">
          <BaseEmpty textKey="tests.unlock.page.empty" />
        </Box>
      ) : (
        <Grid container spacing={3} columns={{ xs: 1, sm: 2, md: 3 }}>
          {unlockItems.map((item) => (
            <Grid size={1} key={item.name}>
              <Card
                variant="outlined"
                className="h-full rounded-lg relative overflow-hidden flex flex-col"
                style={{
                  borderLeft: `4px solid ${getStatusBorderColor(item.status)}`,
                  backgroundColor: 'var(--color-card)',
                }}
              >
                <Box className="p-[5.2px] flex-1">
                  <Box className="flex justify-between items-center">
                    <Typography
                      variant="subtitle1"
                      className="font-semibold text-base text-gray-900 dark:text-gray-100"
                    >
                      {item.name}
                    </Typography>
                    <Tooltip title={t('tests.components.item.actions.test')}>
                      <span>
                        <IconButton
                          size="small"
                          color="primary"
                          disabled={
                            loadingItems.includes(item.name) || isCheckingAll
                          }
                          className="border border-solid border-primary hover:bg-primary/10 dark:hover:bg-primary-dark-mode/10"
                          onClick={() => checkSingleMedia(item.name)}
                        >
                          <RefreshCw 
                            className={`h-4 w-4 ${loadingItems.includes(item.name) ? 'animate-spin' : ''}`}
                          />
                        </IconButton>
                      </span>
                    </Tooltip>
                  </Box>

                  <Box className="flex items-center flex-wrap gap-1">
                    <Chip
                      label={t(STATUS_LABEL_KEYS[item.status] ?? item.status)}
                      color={getStatusColor(item.status)}
                      size="small"
                      icon={getStatusIcon(item.status)}
                      className={item.status === 'Pending' ? 'font-normal' : 'font-bold'}
                    />

                    {item.region && (
                      <Chip
                        label={item.region}
                        size="small"
                        variant="outlined"
                        color="info"
                      />
                    )}
                  </Box>
                </Box>

                <Divider
                  className="mx-1 border-dashed opacity-20"
                />

                <Box className="px-6 py-0.8">
                  <Typography
                    variant="caption"
                    className="block text-gray-500 dark:text-gray-400 text-[0.7rem] text-right"
                  >
                    {item.check_time || '-- --'}
                  </Typography>
                </Box>
              </Card>
            </Grid>
          ))}
        </Grid>
      )}
    </BasePage>
  )
}

export default UnlockPage
