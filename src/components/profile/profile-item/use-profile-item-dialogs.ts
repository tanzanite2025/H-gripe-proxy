import { useLockFn } from 'ahooks'
import { useCallback, useState } from 'react'

import { useEditorDocument } from '@/hooks/ui'
import { readProfileFile, saveProfileFile } from '@/services/cmds'

interface UseProfileItemDialogsParams {
  uid: string
  option?: IProfileOption
  onSave?: (prev?: string, curr?: string) => void | Promise<void>
}

export interface ProfileItemDialogsController {
  fileOpen: boolean
  proxiesOpen: boolean
  scriptOpen: boolean
  confirmOpen: boolean
  qrOpen: boolean
  updateHistoryOpen: boolean
  profileDocument: ReturnType<typeof useEditorDocument>
  scriptDocument: ReturnType<typeof useEditorDocument>
  openFile: () => void
  closeFile: () => void
  openProxies: () => void
  closeProxies: () => void
  openScript: () => void
  closeScript: () => void
  openConfirm: () => void
  closeConfirm: () => void
  openQr: () => void
  closeQr: () => void
  openUpdateHistory: () => void
  closeUpdateHistory: () => void
  handleSaveProfileDocument: () => Promise<void>
  handleSaveScriptDocument: () => Promise<void>
}

export function useProfileItemDialogs({
  uid,
  option,
  onSave,
}: UseProfileItemDialogsParams): ProfileItemDialogsController {
  const [fileOpen, setFileOpen] = useState(false)
  const [proxiesOpen, setProxiesOpen] = useState(false)
  const [scriptOpen, setScriptOpen] = useState(false)
  const [confirmOpen, setConfirmOpen] = useState(false)
  const [qrOpen, setQrOpen] = useState(false)
  const [updateHistoryOpen, setUpdateHistoryOpen] = useState(false)

  const loadProfileDocument = useCallback(() => readProfileFile(uid), [uid])
  const loadScriptDocument = useCallback(
    () => readProfileFile(option?.script ?? ''),
    [option?.script],
  )

  const profileDocument = useEditorDocument({
    open: fileOpen,
    load: loadProfileDocument,
  })
  const scriptDocument = useEditorDocument({
    open: scriptOpen,
    load: loadScriptDocument,
  })

  const openFile = useCallback(() => setFileOpen(true), [])
  const closeFile = useCallback(() => setFileOpen(false), [])
  const openProxies = useCallback(() => setProxiesOpen(true), [])
  const closeProxies = useCallback(() => setProxiesOpen(false), [])
  const openScript = useCallback(() => setScriptOpen(true), [])
  const closeScript = useCallback(() => setScriptOpen(false), [])
  const openConfirm = useCallback(() => setConfirmOpen(true), [])
  const closeConfirm = useCallback(() => setConfirmOpen(false), [])
  const openQr = useCallback(() => setQrOpen(true), [])
  const closeQr = useCallback(() => setQrOpen(false), [])
  const openUpdateHistory = useCallback(() => setUpdateHistoryOpen(true), [])
  const closeUpdateHistory = useCallback(() => setUpdateHistoryOpen(false), [])

  const handleSaveProfileDocument = useLockFn(async () => {
    const currentValue = profileDocument.value
    if (!(await saveProfileFile(uid, currentValue))) {
      await profileDocument.reload()
      return
    }

    await onSave?.(profileDocument.savedValue, currentValue)
    profileDocument.markSaved(currentValue)
  })

  const handleSaveScriptDocument = useLockFn(async () => {
    const scriptUid = option?.script ?? ''
    const currentValue = scriptDocument.value
    if (!(await saveProfileFile(scriptUid, currentValue))) {
      await scriptDocument.reload()
      return
    }

    await onSave?.(scriptDocument.savedValue, currentValue)
    scriptDocument.markSaved(currentValue)
  })

  return {
    fileOpen,
    proxiesOpen,
    scriptOpen,
    confirmOpen,
    qrOpen,
    updateHistoryOpen,
    profileDocument,
    scriptDocument,
    openFile,
    closeFile,
    openProxies,
    closeProxies,
    openScript,
    closeScript,
    openConfirm,
    closeConfirm,
    openQr,
    closeQr,
    openUpdateHistory,
    closeUpdateHistory,
    handleSaveProfileDocument,
    handleSaveScriptDocument,
  }
}
