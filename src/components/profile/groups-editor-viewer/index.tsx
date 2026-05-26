import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
} from '@mui/material'
import { useLockFn } from 'ahooks'
import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { MonacoEditor } from '@/components/base'
import { saveProfileFile } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { useThemeMode } from '@/services/states'
import type { MonacoEditorInstance } from '@/types/monaco'
import getSystem from '@/utils/misc'

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
  const themeMode = useThemeMode()
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
    >
      <DialogTitle>
        <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
          {t('profiles.modals.groupsEditor.title')}
          <Box>
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
          </Box>
        </Box>
      </DialogTitle>

      <DialogContent
        sx={{ display: 'flex', width: 'auto', height: 'calc(100vh - 185px)' }}
      >
        {visualization ? (
          <>
            <GroupForm
              control={control}
              formIns={undefined as any}
              proxyPolicyList={proxyPolicyList}
              proxyProviderList={proxyProviderList}
              interfaceNameList={interfaceNameList}
              translateStrategy={translateStrategy}
              translatePolicy={translatePolicy}
              onPrepend={handlePrepend}
              onAppend={handleAppend}
            />

            <Box
              sx={{
                width: '50%',
                padding: '0 10px',
              }}
            >
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
            </Box>
          </>
        ) : (
          <MonacoEditor
            height="100%"
            language="yaml"
            value={currData}
            theme={themeMode === 'light' ? 'light' : 'vs-dark'}
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
              fontFamily: `Fira Code, JetBrains Mono, Roboto Mono, "Source Code Pro", Consolas, Menlo, Monaco, monospace, "Courier New", "Apple Color Emoji"${
                getSystem() === 'windows' ? ', twemoji mozilla' : ''
              }`,
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
