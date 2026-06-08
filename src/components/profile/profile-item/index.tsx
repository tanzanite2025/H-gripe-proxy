import { useSortable } from '@dnd-kit/sortable'

import { ProfileItemUI } from '../profile-item-ui'
import { ProfileItemDialogs } from './profile-item-dialogs'
import { ProfileItemMenu } from './profile-item-menu'
import { type ProfileItemProps } from './shared'
import { useProfileItemActions } from './use-profile-item-actions'
import { useProfileItemDialogs } from './use-profile-item-dialogs'
import { useProfileItemState } from './use-profile-item-state'

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

  const state = useProfileItemState({
    itemData,
    mutateProfiles,
  })

  const dialogs = useProfileItemDialogs({
    uid: state.uid,
    option: state.option,
    onSave,
  })

  const actions = useProfileItemActions({
    itemData,
    dialogs,
    mutateProfiles,
    setProfileLoading: state.setProfileLoading,
    onEdit,
    onSelect,
    batchMode,
    onSelectionChange,
  })

  return (
    <>
      <ProfileItemUI
        name={state.name}
        description={state.description}
        from={state.from}
        hasUrl={state.hasUrl}
        hasExtra={state.hasExtra}
        selected={selected}
        activating={activating}
        loading={state.loading}
        isDragging={isDragging}
        batchMode={batchMode}
        isSelected={isSelected}
        updated={state.updated}
        showNextUpdate={state.showNextUpdate}
        nextUpdateTime={state.nextUpdateTime}
        upload={state.upload}
        download={state.download}
        total={state.total}
        expire={state.expire}
        progress={state.progress}
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
        onContextMenu={actions.openMenu}
        onUseClick={() => {
          if (activating) {
            return
          }

          actions.handleForceSelect()
        }}
        onDirectUpdateClick={() => {
          if (activating || state.loading) {
            return
          }

          actions.handleDirectUpdate()
        }}
        onProxyUpdateClick={() => {
          if (activating || state.loading) {
            return
          }

          actions.handleProxyUpdate()
        }}
        onEditProxiesClick={() => {
          if (!state.option?.proxies) {
            return
          }

          actions.handleEditProxies()
        }}
        onEditGroupsClick={() => {
          if (!state.option?.groups) {
            return
          }

          actions.handleEditGroups()
        }}
        onShareQrCodeClick={() => {
          if (!state.hasUrl) {
            return
          }

          actions.handleShareQrCode()
        }}
        canEditProxies={!!state.option?.proxies}
        canEditGroups={!!state.option?.groups}
        onToggleUpdateTimeDisplay={state.toggleUpdateTimeDisplay}
        onSelectionChange={onSelectionChange}
      />

      <ProfileItemMenu
        open={actions.menuOpen}
        position={actions.position}
        items={actions.menuItems}
        onClose={actions.closeMenu}
      />

      <ProfileItemDialogs
        itemData={itemData}
        name={state.name}
        option={state.option}
        onSave={onSave}
        onDelete={onDelete}
        dialogs={dialogs}
      />
    </>
  )
}
