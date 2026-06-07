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
  rulesOpen: boolean
  proxiesOpen: boolean
  groupsOpen: boolean
  mergeOpen: boolean
  scriptOpen: boolean
  confirmOpen: boolean
  qrOpen: boolean
  profileDocument: ReturnType<typeof useEditorDocument>
  mergeDocument: ReturnType<typeof useEditorDocument>
  scriptDocument: ReturnType<typeof useEditorDocument>
  openFile: () => void
  closeFile: () => void
  openRules: () => void
  closeRules: () => void
  openProxies: () => void
  closeProxies: () => void
  openGroups: () => void
  closeGroups: () => void
  openMerge: () => void
  closeMerge: () => void
  openScript: () => void
  closeScript: () => void
  openConfirm: () => void
  closeConfirm: () => void
  openQr: () => void
  closeQr: () => void
  handleSaveProfileDocument: () => Promise<void>
  handleSaveMergeDocument: () => Promise<void>
  handleSaveScriptDocument: () => Promise<void>
}

export function useProfileItemDialogs({
  uid,
  option,
  onSave,
}: UseProfileItemDialogsParams): ProfileItemDialogsController {
  const [fileOpen, setFileOpen] = useState(false)
  const [rulesOpen, setRulesOpen] = useState(false)
  const [proxiesOpen, setProxiesOpen] = useState(false)
  const [groupsOpen, setGroupsOpen] = useState(false)
  const [mergeOpen, setMergeOpen] = useState(false)
  const [scriptOpen, setScriptOpen] = useState(false)
  const [confirmOpen, setConfirmOpen] = useState(false)
  const [qrOpen, setQrOpen] = useState(false)

  const loadProfileDocument = useCallback(() => readProfileFile(uid), [uid])
  const loadMergeDocument = useCallback(
    () => readProfileFile(option?.merge ?? ''),
    [option?.merge],
  )
  const loadScriptDocument = useCallback(
    () => readProfileFile(option?.script ?? ''),
    [option?.script],
  )

  const profileDocument = useEditorDocument({
    open: fileOpen,
    load: loadProfileDocument,
  })
  const mergeDocument = useEditorDocument({
    open: mergeOpen,
    load: loadMergeDocument,
  })
  const scriptDocument = useEditorDocument({
    open: scriptOpen,
    load: loadScriptDocument,
  })

  const openFile = useCallback(() => setFileOpen(true), [])
  const closeFile = useCallback(() => setFileOpen(false), [])
  const openRules = useCallback(() => setRulesOpen(true), [])
  const closeRules = useCallback(() => setRulesOpen(false), [])
  const openProxies = useCallback(() => setProxiesOpen(true), [])
  const closeProxies = useCallback(() => setProxiesOpen(false), [])
  const openGroups = useCallback(() => setGroupsOpen(true), [])
  const closeGroups = useCallback(() => setGroupsOpen(false), [])
  const openMerge = useCallback(() => setMergeOpen(true), [])
  const closeMerge = useCallback(() => setMergeOpen(false), [])
  const openScript = useCallback(() => setScriptOpen(true), [])
  const closeScript = useCallback(() => setScriptOpen(false), [])
  const openConfirm = useCallback(() => setConfirmOpen(true), [])
  const closeConfirm = useCallback(() => setConfirmOpen(false), [])
  const openQr = useCallback(() => setQrOpen(true), [])
  const closeQr = useCallback(() => setQrOpen(false), [])

  const handleSaveProfileDocument = useLockFn(async () => {
    const currentValue = profileDocument.value
    if (!(await saveProfileFile(uid, currentValue))) {
      await profileDocument.reload()
      return
    }

    await onSave?.(profileDocument.savedValue, currentValue)
    profileDocument.markSaved(currentValue)
  })

  const handleSaveMergeDocument = useLockFn(async () => {
    const mergeUid = option?.merge ?? ''
    const currentValue = mergeDocument.value
    if (!(await saveProfileFile(mergeUid, currentValue))) {
      await mergeDocument.reload()
      return
    }

    await onSave?.(mergeDocument.savedValue, currentValue)
    mergeDocument.markSaved(currentValue)
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
    rulesOpen,
    proxiesOpen,
    groupsOpen,
    mergeOpen,
    scriptOpen,
    confirmOpen,
    qrOpen,
    profileDocument,
    mergeDocument,
    scriptDocument,
    openFile,
    closeFile,
    openRules,
    closeRules,
    openProxies,
    closeProxies,
    openGroups,
    closeGroups,
    openMerge,
    closeMerge,
    openScript,
    closeScript,
    openConfirm,
    closeConfirm,
    openQr,
    closeQr,
    handleSaveProfileDocument,
    handleSaveMergeDocument,
    handleSaveScriptDocument,
  }
}
