import { useQuery } from '@tanstack/react-query'
import { useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useLocation } from 'react-router'

import { BasePage, DialogRef } from '@/components/base'
import { ProfileHeader } from '@/components/profile/profile-header'
import { ProfileMore } from '@/components/profile/profile-more'
import { ProfileRulesPanel } from '@/components/profile/profile-rules-panel'
import {
  ProfileViewer,
  ProfileViewerRef,
} from '@/components/profile/profile-viewer'
import { ConfigViewer } from '@/components/setting/components/misc/config-editor'
import { Box } from '@/components/tailwind'
import { useProfiles } from '@/hooks/data'
import { useAppRefreshers } from '@/providers/app-data-context'
import { getRuntimeLogs } from '@/services/cmds'
import { debugLog } from '@/utils/misc'

import { ProfileCardsSection } from './profiles-page/profile-cards-section'
import {
  collectPrimaryProfileItems,
} from './profiles-page/profile-item-utils'
import { useProfileActivation } from './profiles-page/use-profile-activation'
import { useProfileBatchSelection } from './profiles-page/use-profile-batch-selection'
import { useProfileChangeListener } from './profiles-page/use-profile-change-listener'
import { useProfilesPageController } from './profiles-page/use-profiles-page-controller'

const ProfilePage = () => {
  const { t } = useTranslation()
  const location = useLocation()
  const { refreshRules, refreshRuleProviders } = useAppRefreshers()

  const [scriptOpen, setScriptOpen] = useState(false)

  const { current } = location.state || {}

  const {
    profiles = {},
    activateSelected,
    patchProfiles,
    mutateProfiles,
    error,
    isStale,
  } = useProfiles()

  const { data: chainLogs = {}, refetch: mutateLogs } = useQuery({
    queryKey: ['getRuntimeLogs'],
    queryFn: getRuntimeLogs,
  })

  const viewerRef = useRef<ProfileViewerRef>(null)
  const configRef = useRef<DialogRef>(null)

  const profileItems = useMemo(() => {
    const items = profiles.items ?? []
    const primaryItems =
      profiles.primaryItems ?? collectPrimaryProfileItems(items)

    if (items.length > 0 && primaryItems.length === 0) {
      debugLog(
        '[profiles] primary profile items were filtered out unexpectedly',
        items,
      )
    }

    return primaryItems
  }, [profiles.items, profiles.primaryItems])

  const {
    activatings,
    setActivatings,
    switchingProfileRef,
    getCurrentActivatings,
    onSelect,
  } = useProfileActivation({
    currentProfileId: current,
    profiles,
    activateSelected,
    patchProfiles,
    mutateProfiles,
    refreshRules,
    refreshRuleProviders,
    mutateLogs,
  })

  const {
    url,
    setUrl,
    disabled,
    loading,
    onEnhance,
    onEmergencyRefresh,
    onImport,
    onDragEnd,
    onDelete,
    onUpdateAll,
    onCopyLink,
  } = useProfilesPageController({
    profiles,
    profileItems,
    mutateProfiles,
    mutateLogs,
    switchingProfileRef,
    getCurrentActivatings,
    setActivatings,
  })

  const currentPrimaryProfileId =
    profiles.currentPrimaryUid ?? profiles.current

  const {
    batchMode,
    selectedProfiles,
    toggleBatchMode,
    toggleProfileSelection,
    selectAllProfiles,
    clearAllSelections,
    isAllSelected,
    getSelectionState,
    deleteSelectedProfiles,
  } = useProfileBatchSelection({
    profileItems,
    currentProfileId: currentPrimaryProfileId,
    setActivatings,
    mutateProfiles,
    mutateLogs,
    onEnhance,
  })

  useProfileChangeListener({ mutateProfiles })

  return (
    <BasePage
      full
      title={t('profiles.page.title')}
      contentStyle={{ height: '100%' }}
    >
      <Box className="flex h-full flex-col overflow-hidden">
        <Box className="shrink-0 px-[10px] pb-1 pt-2">
          <ProfileHeader
            batchMode={batchMode}
            error={error}
            isStale={isStale}
            selectedCount={selectedProfiles.size}
            isAllSelected={isAllSelected}
            getSelectionState={getSelectionState}
            clearAllSelections={clearAllSelections}
            selectAllProfiles={selectAllProfiles}
            toggleBatchMode={toggleBatchMode}
            onUpdateAll={onUpdateAll}
            onOpenConfig={() => configRef.current?.open()}
            onReactivate={() => onEnhance(true)}
            onEmergencyRefresh={onEmergencyRefresh}
            onDeleteSelectedProfiles={deleteSelectedProfiles}
            onOpenScript={() => setScriptOpen(true)}
            url={url}
            setUrl={setUrl}
            disabled={disabled}
            loading={loading}
            onImport={onImport}
            onCopyLink={onCopyLink}
            onCreate={() => viewerRef.current?.create()}
          />
        </Box>

        <ProfileCardsSection
          profileItems={profileItems}
          currentProfileId={currentPrimaryProfileId}
          activatings={activatings}
          batchMode={batchMode}
          selectedProfiles={selectedProfiles}
          mutateProfiles={mutateProfiles}
          onDragEnd={onDragEnd}
          onSelect={onSelect}
          onEdit={(item) => viewerRef.current?.edit(item)}
          onSave={async (item, prev, curr) => {
            if (prev !== curr && currentPrimaryProfileId === item.uid) {
              await onEnhance(false)
            }
          }}
          onDelete={onDelete}
          onToggleSelection={toggleProfileSelection}
        />

        <Box className="flex-[2_0_0] min-h-0">
          <ProfileRulesPanel />
        </Box>
      </Box>

      <ProfileViewer
        ref={viewerRef}
        onChange={async (isActivating) => {
          void mutateProfiles()
          if (isActivating) {
            await onEnhance(false)
          }
        }}
      />
      <ConfigViewer ref={configRef} />

      <ProfileMore
        id="Script"
        open={scriptOpen}
        onClose={() => setScriptOpen(false)}
        logInfo={chainLogs['Script']}
        onSave={async (prev, curr) => {
          if (prev !== curr) {
            await onEnhance(false)
          }
        }}
      />
    </BasePage>
  )
}

export default ProfilePage
