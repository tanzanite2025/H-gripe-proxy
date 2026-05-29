import { Database, RefreshCw } from 'lucide-react'
import { useLockFn } from 'ahooks'
import dayjs from 'dayjs'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { updateRuleProvider } from 'tauri-plugin-mihomo-api'

import { Button } from '@/components/tailwind/Button'
import { Dialog, DialogActions, DialogContent, DialogTitle } from '@/components/tailwind/Dialog'
import { IconButton } from '@/components/tailwind/IconButton'
import { List, ListItem, ListItemText } from '@/components/tailwind/List'
import { useAppRefreshers, useRulesData } from '@/providers/app-data-context'
import { showNotice } from '@/services/notice-service'
import { cn } from '@/utils/cn'

export const ProviderButton = () => {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const { ruleProviders } = useRulesData()
  const { refreshRules, refreshRuleProviders } = useAppRefreshers()
  const [updating, setUpdating] = useState<Record<string, boolean>>({})

  // 检查是否有提供者
  const hasProviders = Object.keys(ruleProviders || {}).length > 0

  // 更新单个规则提供者
  const updateProvider = useLockFn(async (name: string) => {
    try {
      // 设置更新状态
      setUpdating((prev) => ({ ...prev, [name]: true }))

      await updateRuleProvider(name)

      // 刷新数据
      await refreshRules()
      await refreshRuleProviders()

      showNotice.success(
        'rules.feedback.notifications.provider.updateSuccess',
        {
          name,
        },
      )
    } catch (err) {
      showNotice.error('rules.feedback.notifications.provider.updateFailed', {
        name,
        message: String(err),
      })
    } finally {
      // 清除更新状态
      setUpdating((prev) => ({ ...prev, [name]: false }))
    }
  })

  // 更新所有规则提供者
  const updateAllProviders = useLockFn(async () => {
    try {
      // 获取所有provider的名称
      const allProviders = Object.keys(ruleProviders || {})
      if (allProviders.length === 0) {
        showNotice.info('rules.feedback.notifications.provider.none')
        return
      }

      // 设置所有provider为更新中状态
      const newUpdating = allProviders.reduce(
        (acc, key) => {
          acc[key] = true
          return acc
        },
        {} as Record<string, boolean>,
      )
      setUpdating(newUpdating)

      // 改为串行逐个更新所有provider
      for (const name of allProviders) {
        try {
          await updateRuleProvider(name)
          // 每个更新完成后更新状态
          setUpdating((prev) => ({ ...prev, [name]: false }))
        } catch (err) {
          console.error(`更新 ${name} 失败`, err)
          // 继续执行下一个，不中断整体流程
        }
      }

      // 刷新数据
      await refreshRules()
      await refreshRuleProviders()

      showNotice.success('rules.feedback.notifications.provider.allUpdated')
    } catch (err) {
      showNotice.error('rules.feedback.notifications.provider.genericError', {
        message: String(err),
      })
    } finally {
      // 清除所有更新状态
      setUpdating({})
    }
  })

  const handleClose = () => {
    setOpen(false)
  }

  if (!hasProviders) return null

  return (
    <>
      <Button
        variant="outlined"
        size="small"
        startIcon={<Database className="h-4 w-4" />}
        onClick={() => setOpen(true)}
      >
        {t('rules.page.provider.trigger')}
      </Button>

      <Dialog
        open={open}
        onClose={handleClose}
        maxWidth="sm"
        fullWidth
        className="uds-dialog"
      >
        <DialogTitle className="uds-title-h2">
          <div className="flex justify-between items-center">
            <h6 className="text-lg font-semibold uds-title-h2">
              {t('rules.page.provider.dialogTitle')}
            </h6>
            <Button
              variant="contained"
              size="small"
              onClick={updateAllProviders}
            >
              {t('rules.page.provider.actions.updateAll')}
            </Button>
          </div>
        </DialogTitle>

        <DialogContent>
          <List className="py-0 min-h-[250px]">
            {Object.entries(ruleProviders || {})
              .sort()
              .map(([key, item]) => {
                const provider = item
                const time = dayjs(provider.updatedAt)
                const isUpdating = updating[key]

                return (
                  <ListItem
                    key={key}
                    className={cn(
                      'uds-card-container p-0 mb-2 rounded-lg overflow-hidden transition-all duration-200',
                      'bg-white dark:bg-[#24252f]',
                      'hover:bg-primary/10 dark:hover:bg-primary/20'
                    )}
                  >
                    <ListItemText
                      className="px-4 py-2"
                      primary={
                        <div className="flex justify-between items-center">
                          <div className="uds-card-title flex items-center overflow-hidden">
                            <span className="mr-2 truncate" title={key}>{key}</span>
                            <span className="inline-block border border-secondary/50 text-secondary/80 rounded text-[10px] mr-1 px-0.5 leading-tight">
                              {provider.ruleCount}
                            </span>
                          </div>

                          <div className="uds-desc text-sm text-text-secondary whitespace-nowrap ml-2">
                            <small>{t('shared.labels.updateAt')}: </small>
                            {time.fromNow()}
                          </div>
                        </div>
                      }
                      secondary={
                        <div className="flex">
                          <span className="inline-block border border-secondary/50 text-secondary/80 rounded text-[10px] mr-1 px-0.5 leading-tight">
                            {provider.vehicleType}
                          </span>
                          <span className="inline-block border border-secondary/50 text-secondary/80 rounded text-[10px] px-0.5 leading-tight">
                            {provider.behavior}
                          </span>
                        </div>
                      }
                    />
                    <div className="w-px bg-divider self-stretch" />
                    <div className="w-10 flex justify-center items-center">
                      <IconButton
                        size="small"
                        color="primary"
                        onClick={() => updateProvider(key)}
                        disabled={isUpdating}
                        aria-label={t('rules.page.provider.actions.update')}
                        className={cn(
                          isUpdating && 'animate-spin'
                        )}
                        title={t('rules.page.provider.actions.update')}
                      >
                        <RefreshCw className="h-4 w-4" />
                      </IconButton>
                    </div>
                  </ListItem>
                )
              })}
          </List>
        </DialogContent>

        <DialogActions>
          <Button onClick={handleClose} variant="outlined">
            {t('shared.actions.close')}
          </Button>
        </DialogActions>
      </Dialog>
    </>
  )
}
