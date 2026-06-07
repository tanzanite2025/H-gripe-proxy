import { useLockFn } from 'ahooks'
import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { MonacoEditor } from '@/components/base'
import { Button } from '@/components/tailwind/Button'
import { Dialog, DialogActions, DialogContent, DialogTitle } from '@/components/tailwind/Dialog'
import { saveProfileFile } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import type { MonacoEditorInstance } from '@/types/monaco'

import { GroupForm } from './components/group-form'
import { GroupListView } from './components/group-list-view'
import { GroupSearch } from './components/group-search'
import { useGroupData } from './hooks/use-group-data'
import { useGroupDragDrop } from './hooks/use-group-drag-drop'
import { useGroupForm } from './hooks/use-group-form'
import { buildGroupsYaml } from './utils/group-helpers'

interface Props {
  proxiesUid: string
  mergeUid: string
  profileUid: string
  property: string
  open: boolean
  onClose: () => void
  onSave?: (prev?: string, curr?: string) => void
}

export const GroupsEditorViewer = (props: Props) => {
  const { mergeUid, proxiesUid, profileUid, property, open, onClose, onSave } =
    props
  const { t } = useTranslation()
  const editorRef = useRef<MonacoEditorInstance | null>(null)
  const [currData, setCurrData] = useState('')
  const [visualization, setVisualization] = useState(true)
  const [match, setMatch] = useState(() => (_: string) => true)

  // Data management
  const {
    prevData,
    setPrevData,
    groupList,
    proxyPolicyList,
    proxyProviderList,
    prependSeq,
    setPrependSeq,
    appendSeq,
    setAppendSeq,
    deleteSeq,
    setDeleteSeq,
    interfaceNameList,
    fetchContent,
  } = useGroupData({
    mergeUid,
    proxiesUid,
    profileUid,
    property,
    open,
    visualization,
    currData,
    setCurrData,
  })

  // Drag and drop
  const { sensors, onPrependDragEnd, onAppendDragEnd } = useGroupDragDrop({
    prependSeq,
    appendSeq,
    setPrependSeq,
    setAppendSeq,
  })

  // Form management
  const {
    control,
    translateStrategy,
    translatePolicy,
    handlePrepend,
    handleAppend,
  } = useGroupForm({
    prependSeq,
    setPrependSeq,
    appendSeq,
    setAppendSeq,
    groupList,
  })

  // Cleanup editor on unmount
  useEffect(() => {
    return () => {
      editorRef.current?.dispose()
      editorRef.current = null
    }
  }, [])

  // Delete handlers
  const handlePrependDelete = useCallback(
    (name: string) => {
      setPrependSeq(prependSeq.filter((v) => v.name !== name))
    },
    [prependSeq, setPrependSeq],
  )

  const handleAppendDelete = useCallback(
    (name: string) => {
      setAppendSeq(appendSeq.filter((v) => v.name !== name))
    },
    [appendSeq, setAppendSeq],
  )

  const handleGroupToggleDelete = useCallback(
    (name: string) => {
      if (deleteSeq.includes(name)) {
        setDeleteSeq(deleteSeq.filter((v) => v !== name))
      } else {
        setDeleteSeq((prev) => [...prev, name])
      }
    },
    [deleteSeq, setDeleteSeq],
  )

  // Save handler
  const handleSave = useLockFn(async () => {
    try {
      const nextData = visualization
        ? buildGroupsYaml(prependSeq, appendSeq, deleteSeq)
        : currData

      if (visualization) {
        setCurrData(nextData)
      }

      if (!(await saveProfileFile(property, nextData))) {
        await fetchContent()
        onClose()
        return
      }
      showNotice.success('shared.feedback.notifications.saved')
      setPrevData(nextData)
      onSave?.(prevData, nextData)
      onClose()
    } catch (err) {
      showNotice.error(err)
    }
  })

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth="xl"
      fullWidth
      disableEnforceFocus={!visualization}
      slotProps={{ paper: { className: 'max-h-[95vh]' } }}
    >
      <DialogTitle>
        <div className="flex justify-between">
          {t('profiles.modals.groupsEditor.title')}
          <div>
            <Button
              variant="contained"
              size="small"
              onClick={() => {
                setVisualization((prev) => !prev)
              }}
            >
              {visualization
                ? t('shared.editorModes.advanced')
                : t('shared.editorModes.visualization')}
            </Button>
          </div>
        </div>
      </DialogTitle>

      <DialogContent className="flex h-[calc(100vh-185px)] w-auto">
        {visualization ? (
          <>
            <GroupForm
              control={control}
              proxyPolicyList={proxyPolicyList}
              proxyProviderList={proxyProviderList}
              interfaceNameList={interfaceNameList}
              translateStrategy={translateStrategy}
              translatePolicy={translatePolicy}
              onPrepend={handlePrepend}
              onAppend={handleAppend}
            />

            <div className="w-1/2 px-2.5">
              <GroupSearch onSearch={(match) => setMatch(() => match)} />
              <GroupListView
                prependSeq={prependSeq}
                groupList={groupList}
                appendSeq={appendSeq}
                deleteSeq={deleteSeq}
                match={match}
                sensors={sensors}
                onPrependDragEnd={onPrependDragEnd}
                onAppendDragEnd={onAppendDragEnd}
                onPrependDelete={handlePrependDelete}
                onAppendDelete={handleAppendDelete}
                onGroupToggleDelete={handleGroupToggleDelete}
              />
            </div>
          </>
        ) : (
          <MonacoEditor
            height="100%"
            language="yaml"
            value={currData}
            theme='vs-dark'
            onMount={(editorInstance) => {
              editorRef.current = editorInstance
            }}
            options={{
              tabSize: 2,
              minimap: {
                enabled: document.documentElement.clientWidth >= 1500,
              },
              mouseWheelZoom: true,
              quickSuggestions: {
                strings: true,
                comments: true,
                other: true,
              },
              padding: {
                top: 33,
              },
              fontFamily:
                'Josefin Sans, YouSheBiaoTiHei, twemoji mozilla, Segoe UI Emoji, -apple-system, BlinkMacSystemFont, Segoe UI, Microsoft YaHei UI, Microsoft YaHei, Roboto, Helvetica Neue, Arial, sans-serif',
              fontLigatures: false,
              smoothScrolling: true,
            }}
            onChange={(value) => setCurrData(value ?? '')}
          />
        )}
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose} variant="outlined">
          {t('shared.actions.cancel')}
        </Button>

        <Button onClick={handleSave} variant="contained">
          {t('shared.actions.save')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
