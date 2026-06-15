import { useLockFn } from 'ahooks'
import { useCallback, useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/tailwind/Button'
import {
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
} from '@/components/tailwind/Dialog'
import { saveProfileFile } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import type { MonacoEditorInstance } from '@/types/monaco'

import { ProxiesYamlEditorPane } from './proxies-yaml-editor-pane'
import { ProxyVisualizationPane } from './proxy-visualization-pane'
import type { ProxiesEditorViewerProps } from './types'
import { useProxiesEditorState } from './use-proxies-editor-state'

export const ProxiesEditorViewer = ({
  profileUid,
  property,
  open,
  onClose,
  onSave,
}: ProxiesEditorViewerProps) => {
  const { t } = useTranslation()
  const editorRef = useRef<MonacoEditorInstance | null>(null)
  const state = useProxiesEditorState({
    open,
    profileUid,
    property,
  })

  useEffect(() => {
    return () => {
      editorRef.current?.dispose()
      editorRef.current = null
    }
  }, [])

  const handleEditorMount = useCallback(
    (editorInstance: MonacoEditorInstance) => {
      editorRef.current = editorInstance
    },
    [],
  )

  const handleSave = useLockFn(async () => {
    try {
      if (!(await saveProfileFile(property, state.currData))) {
        await state.reloadContent()
        onClose()
        return
      }

      showNotice.success('shared.feedback.notifications.saved')
      onSave?.(state.prevData, state.currData)
      onClose()
    } catch (error) {
      showNotice.error(error)
    }
  })

  return (
    <Dialog open={open} onClose={onClose} maxWidth="xl" fullWidth>
      <DialogTitle>
        <div className="flex justify-between">
          <span>{t('profiles.modals.proxiesEditor.title')}</span>
          <Button
            variant="primary"
            size="small"
            onClick={state.toggleVisualization}
          >
            {state.visualization
              ? t('shared.editorModes.advanced')
              : t('shared.editorModes.visualization')}
          </Button>
        </div>
      </DialogTitle>

      <DialogContent className="flex h-[calc(100vh-185px)] w-auto">
        {state.visualization ? (
          <ProxyVisualizationPane
            proxyUri={state.proxyUri}
            sections={state.sections}
            importPlaceholder={t(
              'profiles.modals.proxiesEditor.placeholders.multiUri',
            )}
            prependLabel={t('profiles.modals.proxiesEditor.actions.prepend')}
            appendLabel={t('profiles.modals.proxiesEditor.actions.append')}
            onProxyUriChange={state.handleProxyUriChange}
            onSearchChange={state.handleSearchChange}
            onPrependImport={() => {
              void state.handlePrependImport()
            }}
            onAppendImport={() => {
              void state.handleAppendImport()
            }}
            onPrependDelete={state.handlePrependDelete}
            onAppendDelete={state.handleAppendDelete}
            onOriginalDeleteToggle={state.handleOriginalDeleteToggle}
            onPrependDragEnd={state.handlePrependDragEnd}
            onAppendDragEnd={state.handleAppendDragEnd}
          />
        ) : (
          <ProxiesYamlEditorPane
            currData={state.currData}
            onChange={state.handleYamlChange}
            onMount={handleEditorMount}
          />
        )}
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose} variant="outlined">
          {t('shared.actions.cancel')}
        </Button>
        <Button onClick={handleSave} variant="primary">
          {t('shared.actions.save')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
