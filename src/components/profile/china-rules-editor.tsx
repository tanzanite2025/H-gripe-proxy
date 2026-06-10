import { useLockFn } from 'ahooks'
import { useCallback } from 'react'

import { useEditorDocument } from '@/hooks/ui'
import { readChinaRulesFile, saveChinaRulesFile } from '@/services/cmds'

import { EditorViewer } from './editor-viewer'

interface ChinaRulesEditorProps {
  open: boolean
  onClose: () => void
  onSave?: (prev?: string, curr?: string) => void | Promise<void>
}

export const ChinaRulesEditor = ({
  open,
  onClose,
  onSave,
}: ChinaRulesEditorProps) => {
  const loadDocument = useCallback(() => readChinaRulesFile(), [])
  const document = useEditorDocument({
    open,
    load: loadDocument,
  })

  const handleSave = useLockFn(async () => {
    const currentValue = document.value
    if (!(await saveChinaRulesFile(currentValue))) {
      await document.reload()
      return
    }

    await onSave?.(document.savedValue, currentValue)
    document.markSaved(currentValue)
  })

  if (!open) {
    return null
  }

  return (
    <EditorViewer
      open={true}
      title="china rules"
      value={document.value}
      language="yaml"
      path="global:china-rules.yaml"
      loading={document.loading}
      dirty={document.dirty}
      onChange={document.setValue}
      onSave={handleSave}
      onClose={onClose}
    />
  )
}
