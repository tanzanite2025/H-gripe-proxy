import { useLockFn, useRequest } from 'ahooks'
import { forwardRef, useImperativeHandle, useMemo, useState, type ChangeEvent } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog, Switch } from '@/components/base'
import { Box, Button, Divider, List, ListItem, TextField } from '@/components/tailwind'
import { Trash2 } from '@/components/tailwind/icons'
import { useClash } from '@/hooks/data'
import { restartCore } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

// 定义开发环境的URL列表
// 这些URL在开发模式下会被自动包含在允许的来源中
// 在生产环境中，这些URL会被过滤掉
// 这样可以确保在生产环境中不会意外暴露开发环境的URL
const DEV_URLS = [
  'tauri://localhost',
  'http://tauri.localhost',
  'http://localhost:3000',
]

// 获取完整的源列表，包括开发URL
const getFullOrigins = (origins: string[]) => {
  // 合并现有源和开发URL，并去重
  const allOrigins = [...origins, ...DEV_URLS]
  const uniqueOrigins = [...new Set(allOrigins)]
  return uniqueOrigins
}

// 过滤基础URL(确保后续添加)
const filterBaseOriginsForUI = (origins: string[]) => {
  return origins.filter((origin: string) => !DEV_URLS.includes(origin.trim()))
}



interface ClashHeaderConfigingRef {
  open: () => void
  close: () => void
}

export const HeaderConfiguration = forwardRef<ClashHeaderConfigingRef>(
  (props, ref) => {
    const { t } = useTranslation()
    const { clash, mutateClash, patchClash } = useClash()
    const [open, setOpen] = useState(false)

    // CORS配置状态管理
    const [corsConfig, setCorsConfig] = useState<{
      allowPrivateNetwork: boolean
      allowOrigins: string[]
    }>(() => {
      const cors = clash?.['external-controller-cors']
      const origins = cors?.['allow-origins'] ?? []
      return {
        allowPrivateNetwork: cors?.['allow-private-network'] ?? true,
        allowOrigins: filterBaseOriginsForUI(origins),
      }
    })

    // 处理CORS配置变更
    const handleCorsConfigChange = (
      key: 'allowPrivateNetwork' | 'allowOrigins',
      value: boolean | string[],
    ) => {
      setCorsConfig((prev) => ({
        ...prev,
        [key]: value,
      }))
    }

    // 添加新的允许来源
    const handleAddOrigin = () => {
      handleCorsConfigChange('allowOrigins', [...corsConfig.allowOrigins, ''])
    }

    // 更新允许来源列表中的某一项
    const handleUpdateOrigin = (index: number, value: string) => {
      const newOrigins = [...corsConfig.allowOrigins]
      newOrigins[index] = value
      handleCorsConfigChange('allowOrigins', newOrigins)
    }

    // 删除允许来源列表中的某一项
    const handleDeleteOrigin = (index: number) => {
      const newOrigins = [...corsConfig.allowOrigins]
      newOrigins.splice(index, 1)
      handleCorsConfigChange('allowOrigins', newOrigins)
    }

    // 保存配置请求
    const { loading, run: saveConfig } = useRequest(
      async () => {
        // 保存时使用完整的源列表（包括开发URL）
        const fullOrigins = getFullOrigins(corsConfig.allowOrigins)

        await patchClash({
          'external-controller-cors': {
            'allow-private-network': corsConfig.allowPrivateNetwork,
            'allow-origins': fullOrigins.filter(
              (origin: string) => origin.trim() !== '',
            ),
          },
        })
        await restartCore()
        await mutateClash()
      },
      {
        manual: true,
        onSuccess: () => {
          setOpen(false)
          showNotice.success('shared.feedback.notifications.common.saveSuccess')
        },
        onError: () => {
          showNotice.error('shared.feedback.notifications.common.saveFailed')
        },
      },
    )

    useImperativeHandle(ref, () => ({
      open: () => {
        const cors = clash?.['external-controller-cors']
        const origins = cors?.['allow-origins'] ?? []
        setCorsConfig({
          allowPrivateNetwork: cors?.['allow-private-network'] ?? true,
          allowOrigins: filterBaseOriginsForUI(origins),
        })
        setOpen(true)
      },
      close: () => setOpen(false),
    }))

    const handleSave = useLockFn(async () => {
      await saveConfig()
    })

    const originEntries = useMemo(() => {
      const counts: Record<string, number> = {}
      return corsConfig.allowOrigins.map((origin, index) => {
        const occurrence = (counts[origin] = (counts[origin] ?? 0) + 1)
        const keyBase = origin || 'origin'
        return {
          origin,
          index,
          key: `${keyBase}-${occurrence}`,
        }
      })
    }, [corsConfig.allowOrigins])

    return (
      <BaseDialog
        open={open}
        title={t('settings.sections.externalCors.title')}
        panelStyle={{ width: 650 }}
        okBtn={loading ? t('shared.statuses.saving') : t('shared.actions.save')}
        cancelBtn={t('shared.actions.cancel')}
        onClose={() => setOpen(false)}
        onCancel={() => setOpen(false)}
        onOk={handleSave}
      >
        <List className="p-2">
          <ListItem className="py-2 px-0">
            <Box className="flex justify-between items-center w-full">
              <span className="font-normal">
                {t('settings.sections.externalCors.fields.allowPrivateNetwork')}
              </span>
              <Switch
                checked={corsConfig.allowPrivateNetwork}
                onCheckedChange={(checked) =>
                  handleCorsConfigChange(
                    'allowPrivateNetwork',
                    checked,
                  )
                }
              />
            </Box>
          </ListItem>

          <Divider className="my-4" />

          <ListItem className="py-2 px-0">
            <div className="w-full">
              <div className="mb-4 font-bold">
                {t('settings.sections.externalCors.fields.allowedOrigins')}
              </div>
              {originEntries.map(({ origin, index, key }) => (
                <div key={key} className="flex items-center mb-4">
                  <TextField
                    fullWidth
                    size="small"
                    className="text-sm mr-8"
                    value={origin}
                    onChange={(e: ChangeEvent<HTMLInputElement>) => handleUpdateOrigin(index, e.target.value)}
                    placeholder={t(
                      'settings.sections.externalCors.placeholders.origin',
                    )}
                  />
                  <Button
                    variant="danger"
                    size="small"
                    onClick={() => handleDeleteOrigin(index)}
                    disabled={corsConfig.allowOrigins.length <= 0}
                    className="rounded-lg shadow-sm hover:shadow-md transition-all"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              ))}
              <Button
                variant="primary"
                size="small"
                onClick={handleAddOrigin}
                className="rounded-lg shadow-sm hover:shadow-md transition-all"
              >
                {t('settings.sections.externalCors.actions.add')}
              </Button>

              <div className="mt-6 p-4 bg-gray-100 rounded">
                <div className="text-gray-600 text-xs italic">
                  {t('settings.sections.externalCors.messages.alwaysIncluded', {
                    urls: DEV_URLS.join(', '),
                  })}
                </div>
              </div>
            </div>
          </ListItem>
        </List>
      </BaseDialog>
    )
  },
)
