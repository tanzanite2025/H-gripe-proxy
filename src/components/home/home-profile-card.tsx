import { useLockFn } from 'ahooks'
import dayjs from 'dayjs'
import { useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'

import { Button } from '@/components/tailwind/Button'
import { LinearProgress } from '@/components/tailwind/LinearProgress'
import { useAppRefreshers } from '@/providers/app-data-context'
import { openWebUrl, updateProfile } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import parseTraffic from '@/utils/format'

import { EnhancedCard } from './enhanced-card'

// 辅助函数解析URL和过期时间
const parseUrl = (url?: string) => {
  if (!url) return '-'
  if (url.startsWith('http')) return new URL(url).host
  return 'local'
}

const parseExpire = (expire?: number) => {
  if (!expire) return '-'
  return dayjs(expire * 1000).format('YYYY-MM-DD')
}

// 使用类型定义，而不是导入
interface ProfileExtra {
  upload: number
  download: number
  total: number
  expire: number
}

interface ProfileItem {
  uid: string
  type?: 'local' | 'remote' | 'merge' | 'script'
  name?: string
  desc?: string
  file?: string
  url?: string
  updated?: number
  extra?: ProfileExtra
  home?: string
  option?: any
}

interface HomeProfileCardProps {
  current: ProfileItem | null | undefined
  onProfileUpdated?: () => void
}

// 提取独立组件减少主组件复杂度
const ProfileDetails = ({
  current,
  onUpdateProfile,
}: {
  current: ProfileItem
  onUpdateProfile: () => void
}) => {
  const { t } = useTranslation()

  const usedTraffic = useMemo(() => {
    if (!current.extra) return 0
    return current.extra.upload + current.extra.download
  }, [current.extra])

  const trafficPercentage = useMemo(() => {
    if (!current.extra || !current.extra.total || current.extra.total <= 0)
      return 0
    return Math.min(Math.round((usedTraffic / current.extra.total) * 100), 100)
  }, [current.extra, usedTraffic])

  return (
    <div>
      <div className="space-y-4">
        {current.url && (
          <div className="flex items-center gap-2">
            <p className="text-sm text-text-secondary flex items-center">
              <span className="flex-shrink-0">{t('shared.labels.from')}: </span>
              {current.home ? (
                <button
                  onClick={() => current.home && openWebUrl(current.home)}
                  className="inline-flex items-center min-w-0 max-w-[calc(100%-40px)] ml-1 font-medium text-primary hover:underline"
                  title={parseUrl(current.url)}
                >
                  <span className="overflow-hidden text-ellipsis whitespace-nowrap min-w-0 flex-1">
                    {parseUrl(current.url)}
                  </span>
                </button>
              ) : (
                <span
                  className="overflow-hidden text-ellipsis whitespace-nowrap min-w-0 flex-1 ml-1 font-medium"
                  title={parseUrl(current.url)}
                >
                  {parseUrl(current.url)}
                </span>
              )}
            </p>
          </div>
        )}

        {current.updated && (
          <div className="flex items-center gap-2">
            <p
              className="text-sm text-text-secondary cursor-pointer"
              onClick={onUpdateProfile}
            >
              {t('shared.labels.updateTime')}:{' '}
              <span className="font-medium">
                {dayjs(current.updated * 1000).format('YYYY-MM-DD HH:mm')}
              </span>
            </p>
          </div>
        )}

        {current.extra && (
          <>
            <div className="flex items-center gap-2">
              <p className="text-sm text-text-secondary">
                {t('shared.labels.usedTotal')}:{' '}
                <span className="font-medium">
                  <span className="uds-mono">{parseTraffic(usedTraffic)}</span> /{' '}
                  <span className="uds-mono">
                    {parseTraffic(current.extra.total)}
                  </span>
                </span>
              </p>
            </div>

            {current.extra.expire > 0 && (
              <div className="flex items-center gap-2">
                <p className="text-sm text-text-secondary">
                  {t('shared.labels.expireTime')}:{' '}
                  <span className="font-medium">
                    {parseExpire(current.extra.expire)}
                  </span>
                </p>
              </div>
            )}

            <div className="mt-2">
              <p className="uds-mono text-xs text-text-secondary mb-1">
                {trafficPercentage}%
              </p>
              <LinearProgress
                variant="determinate"
                value={trafficPercentage}
                className="h-2 rounded"
              />
            </div>
          </>
        )}
      </div>
    </div>
  )
}

// 提取空配置组件
const EmptyProfile = ({ onClick }: { onClick: () => void }) => {
  const { t } = useTranslation()

  return (
    <div
      className="flex flex-col items-center justify-center py-6 cursor-pointer hover:bg-action-hover rounded-lg"
      onClick={onClick}
    >
      <h6 className="text-lg font-semibold uds-card-title mb-2">
        {t('profiles.page.actions.import')} {t('profiles.page.title')}
      </h6>
      <p className="text-sm text-text-secondary">
        {t('profiles.components.card.labels.clickToImport')}
      </p>
    </div>
  )
}

export const HomeProfileCard = ({
  current,
  onProfileUpdated,
}: HomeProfileCardProps) => {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { refreshAll } = useAppRefreshers()

  const onUpdateProfile = useLockFn(async () => {
    if (!current?.uid) return

    try {
      await updateProfile(current.uid, current.option)
      onProfileUpdated?.()

      // 刷新首页数据
      refreshAll()
    } catch (err) {
      showNotice.error(err, 3000)
    }
  })

  // 导航到订阅页面
  const goToProfiles = useCallback(() => {
    navigate('/profile')
  }, [navigate])

  // 卡片标题
  const cardTitle = useMemo(() => {
    if (!current) return t('profiles.page.title')

    if (!current.home) return current.name

    return (
      <button
        className="uds-card-title text-inherit no-underline flex items-center min-w-0 max-w-full font-medium text-lg hover:underline"
        onClick={() => current.home && openWebUrl(current.home)}
        title={current.name}
      >
        <span className="overflow-hidden text-ellipsis whitespace-nowrap flex-1">
          {current.name}
        </span>
      </button>
    )
  }, [current, t])

  // 卡片操作按钮
  const cardAction = useMemo(() => {
    if (!current) return null

    return (
      <Button
        variant="outlined"
        size="small"
        onClick={goToProfiles}
        className="rounded-xl"
      >
        {t('layout.components.navigation.tabs.profiles')}
      </Button>
    )
  }, [current, goToProfiles, t])

  return (
    <EnhancedCard
      title={cardTitle}
      icon={null}
      iconColor="info"
      action={cardAction}
    >
      {current ? (
        <ProfileDetails current={current} onUpdateProfile={onUpdateProfile} />
      ) : (
        <EmptyProfile onClick={goToProfiles} />
      )}
    </EnhancedCard>
  )
}
