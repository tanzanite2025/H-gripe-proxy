import {
  DndContext,
  DragOverlay,
  KeyboardSensor,
  PointerSensor,
  type DragEndEvent,
  closestCenter,
  useSensor,
  useSensors,
} from '@dnd-kit/core'
import { SortableContext, sortableKeyboardCoordinates } from '@dnd-kit/sortable'

import { BaseEmpty } from '@/components/base'
import { ProfileItem } from '@/components/profile/profile-item'
import { Box, Grid } from '@/components/tailwind'

interface Props {
  profileItems: IProfileItem[]
  currentProfileId?: string
  activatings: string[]
  batchMode: boolean
  selectedProfiles: Set<string>
  mutateProfiles: () => Promise<unknown>
  onDragEnd: (event: DragEndEvent) => void | Promise<void>
  onSelect: (uid: string, force: boolean) => void | Promise<void>
  onEdit: (item: IProfileItem) => void
  onSave: (item: IProfileItem, prev?: string, curr?: string) => Promise<void>
  onDelete: (uid: string) => void | Promise<void>
  onToggleSelection: (uid: string) => void
}

export function ProfileCardsSection({
  profileItems,
  currentProfileId,
  activatings,
  batchMode,
  selectedProfiles,
  mutateProfiles,
  onDragEnd,
  onSelect,
  onEdit,
  onSave,
  onDelete,
  onToggleSelection,
}: Props) {
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  )

  if (profileItems.length === 0) {
    return (
      <Box className="flex-[3_0_0] min-h-0 px-[10px] pb-6 pt-4">
        <Box className="h-full rounded-2xl border border-dashed border-white/10 bg-white/5">
          <BaseEmpty />
        </Box>
      </Box>
    )
  }

  return (
    <Box className="flex-[3_0_0] overflow-y-auto min-h-0">
      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragEnd={onDragEnd}
      >
        <Box className="pl-[10px] pr-[10px] pt-4">
          <Box className="mb-6">
            <Grid container spacing={{ xs: 3, lg: 3 }}>
              <SortableContext items={profileItems.map((item) => item.uid)}>
                {profileItems.map((item) => (
                  <Grid item xs={12} sm={6} lg={4} key={item.uid}>
                    <ProfileItem
                      id={item.uid}
                      selected={currentProfileId === item.uid}
                      activating={activatings.includes(item.uid)}
                      itemData={item}
                      mutateProfiles={async () => {
                        await mutateProfiles()
                      }}
                      onSelect={(force) => onSelect(item.uid, force)}
                      onEdit={() => onEdit(item)}
                      onSave={(prev, curr) => onSave(item, prev, curr)}
                      onDelete={() => {
                        if (batchMode) {
                          onToggleSelection(item.uid)
                        } else {
                          void onDelete(item.uid)
                        }
                      }}
                      batchMode={batchMode}
                      isSelected={selectedProfiles.has(item.uid)}
                      onSelectionChange={() => onToggleSelection(item.uid)}
                    />
                  </Grid>
                ))}
              </SortableContext>
            </Grid>
          </Box>
        </Box>
        <DragOverlay />
      </DndContext>
    </Box>
  )
}
