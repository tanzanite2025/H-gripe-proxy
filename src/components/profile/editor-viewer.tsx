import {
  CloseFullscreenRounded,
  ContentPasteRounded,
  FormatPaintRounded,
  OpenInFullRounded,
} from '@mui/icons-material'

import { Button } from '@/components/tailwind/Button'
import {
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
} from '@/components/tailwind/Dialog'
import { IconButton } from '@/components/tailwind/IconButton'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useLockFn } from 'ahooks'
import { type ReactNode, useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseLoadingOverlay, MonacoEditor } from '@/components/base'
import { showNotice } from '@/services/notice-service'
import { useThemeMode } from '@/services/states'
import type { MonacoEditorInstance, MonacoMarker } from '@/types/monaco'
import debounce from '@/utils/misc/debounce'
import getSystem from '@/utils/misc'

const appWindow = getCurrentWebviewWindow()

export type EditorLanguage = 'yaml' | 'javascript' | 'css'

export interface EditorViewerProps {
  open: boolean
  title?: string | ReactNode
  value: string
  language: EditorLanguage
  path: string
  readOnly?: boolean
  loading?: boolean
  dirty?: boolean
  saveDisabled?: boolean
  onChange?: (value: string) => void
  onSave?: () => void | Promise<void>
  onClose: () => void
  onValidate?: (markers: MonacoMarker[]) => void
}

export const EditorViewer = ({
  open,
  title,
  value,
  language,
  path,
  readOnly = false,
  loading = false,
  dirty,
  saveDisabled = false,
  onChange,
  onSave,
  onClose,
  onValidate,
}: EditorViewerProps) => {
  const { t } = useTranslation()
  const themeMode = useThemeMode()
  const [isMaximized, setIsMaximized] = useState(false)
  const editorRef = useRef<MonacoEditorInstance | null>(null)

  const resolvedTitle = title ?? t('profiles.components.menu.editFile')
  const disableSave = loading || saveDisabled || dirty === false

  const syncEditorValue = useCallback(() => {
    const model = editorRef.current?.getModel()
    if (model && model.getValue() !== value) {
      model.setValue(value)
    }
  }, [value])

  const syncMaximizedState = useCallback(async () => {
    try {
      setIsMaximized(await appWindow.isMaximized())
    } catch {
      setIsMaximized(false)
    }
  }, [])

  const handleSave = useLockFn(async () => {
    try {
      if (!readOnly) {
        await onSave?.()
      }
      onClose()
    } catch (error) {
      showNotice.error(error)
    }
  })

  const handleClose = () => {
    try {
      onClose()
    } catch (error) {
      showNotice.error(error)
    }
  }

  const handlePaste = useLockFn(async () => {
    try {
      if (readOnly || loading || !editorRef.current) return

      const text = await navigator.clipboard.readText()
      if (!text) return

      const editorInstance = editorRef.current
      const model = editorInstance.getModel()
      const selections = editorInstance.getSelections()
      if (!model || !selections || selections.length === 0) return

      editorInstance.pushUndoStop()
      editorInstance.executeEdits(
        'explicit-paste',
        selections.map((selection) => ({
          range: selection,
          text,
          forceMoveMarkers: true,
        })),
      )
      editorInstance.pushUndoStop()
      editorInstance.focus()
    } catch (error) {
      showNotice.error(error)
    }
  })

  const handleFormat = useLockFn(async () => {
    try {
      if (loading) return
      await editorRef.current?.getAction('editor.action.formatDocument')?.run()
    } catch (error) {
      showNotice.error(error)
    }
  })

  const handleToggleMaximize = useLockFn(async () => {
    try {
      await appWindow.toggleMaximize()
      await syncMaximizedState()
      editorRef.current?.layout()
    } catch (error) {
      showNotice.error(error)
    }
  })

  useEffect(() => {
    if (!open) return
    void syncMaximizedState()
  }, [open, syncMaximizedState])

  useEffect(() => {
    if (!open || loading) return
    syncEditorValue()
  }, [loading, open, syncEditorValue])

  useEffect(() => {
    if (!open) return

    const onResized = debounce(() => {
      void syncMaximizedState()
      try {
        editorRef.current?.layout()
      } catch {
        // Ignore transient layout errors during window transitions.
      }
    }, 100)

    const unlistenResized = appWindow.onResized(onResized)

    return () => {
      unlistenResized.then((unlisten) => unlisten())
    }
  }, [open, syncMaximizedState])

  useEffect(() => {
    return () => {
      editorRef.current?.dispose()
      editorRef.current = null
    }
  }, [])

  return (
    <Dialog open={open} onClose={handleClose} maxWidth="xl" fullWidth>
      <DialogTitle>{resolvedTitle}</DialogTitle>

      <DialogContent className="w-auto h-[calc(100vh-185px)] flex flex-col overflow-hidden">
        <div style={{ position: 'relative', flex: '1 1 auto', minHeight: 0 }}>
          <BaseLoadingOverlay isLoading={loading} />
          {!loading && (
            <MonacoEditor
              height="100%"
              path={path}
              value={value}
              language={language}
              theme={themeMode === 'light' ? 'light' : 'vs-dark'}
              loading={null}
              saveViewState
              keepCurrentModel={false}
              onMount={(editorInstance) => {
                editorRef.current = editorInstance
                syncEditorValue()
              }}
              onChange={(nextValue) => onChange?.(nextValue ?? '')}
              onValidate={onValidate}
              options={{
                automaticLayout: true,
                tabSize: 2,
                minimap: {
                  enabled:
                    typeof document !== 'undefined' &&
                    document.documentElement.clientWidth >= 1500,
                },
                mouseWheelZoom: true,
                readOnly,
                readOnlyMessage: {
                  value: t('profiles.modals.editor.messages.readOnly'),
                },
                renderValidationDecorations: 'on',
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
            />
          )}
        </div>

        <div className="absolute left-3.5 bottom-2 flex gap-1">
          {!readOnly && (
            <>
              <IconButton
                size="medium"
                title={t('profiles.page.importForm.actions.paste')}
                disabled={loading}
                onClick={() => {
                  void handlePaste()
                }}
              >
                <ContentPasteRounded fontSize="inherit" />
              </IconButton>
              <IconButton
                size="medium"
                title={t('profiles.modals.editor.actions.format')}
                disabled={loading}
                onClick={() => {
                  void handleFormat()
                }}
              >
                <FormatPaintRounded fontSize="inherit" />
              </IconButton>
            </>
          )}
          <IconButton
            size="medium"
            title={t(
              isMaximized ? 'shared.window.minimize' : 'shared.window.maximize',
            )}
            onClick={() => {
              void handleToggleMaximize()
            }}
          >
            {isMaximized ? <CloseFullscreenRounded /> : <OpenInFullRounded />}
          </IconButton>
        </div>
      </DialogContent>

      <DialogActions>
        <Button onClick={handleClose} variant="outlined">
          {t(readOnly ? 'shared.actions.close' : 'shared.actions.cancel')}
        </Button>
        {!readOnly && (
          <Button
            onClick={() => {
              void handleSave()
            }}
            variant="contained"
            disabled={disableSave}
          >
            {t('shared.actions.save')}
          </Button>
        )}
      </DialogActions>
    </Dialog>
  )
}
