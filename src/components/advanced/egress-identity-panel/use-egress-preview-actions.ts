import { useState } from 'react'

import type { CoordinatorStatus } from '@/services/coordinator'
import {
  egressIdentityAssignMatch,
  egressIdentityClearAssignment,
  egressIdentityPreviewMatch,
  type EgressPreviewRequest,
  type ResolvedEgressIdentity,
} from '@/services/egress-identity'
import { showNotice } from '@/services/notice-service'


import {
  emptyEgressIdentityPreviewForm,
  splitList,
  type EgressIdentityPreviewFormState,
} from './shared'

interface UseEgressPreviewActionsParams {
  onRefreshStatus: () => Promise<CoordinatorStatus | null>
}

export function useEgressPreviewActions({
  onRefreshStatus,
}: UseEgressPreviewActionsParams) {
  const [previewResult, setPreviewResult] =
    useState<ResolvedEgressIdentity | null>(null)
  const [previewLoading, setPreviewLoading] = useState(false)
  const [assignLoading, setAssignLoading] = useState(false)
  const [assignmentsLoading, setAssignmentsLoading] = useState(false)
  const [previewForm, setPreviewForm] =
    useState<EgressIdentityPreviewFormState>(emptyEgressIdentityPreviewForm)

  const refreshAssignments = async () => {
    setAssignmentsLoading(true)
    try {
      await onRefreshStatus()
    } finally {
      setAssignmentsLoading(false)
    }
  }

  const buildPreviewRequest = (): EgressPreviewRequest => ({
    process_name: previewForm.process_name.trim() || undefined,
    exe_path: previewForm.exe_path.trim() || undefined,
    shortcut_id: previewForm.shortcut_id.trim() || undefined,
    domain: previewForm.domain.trim() || undefined,
    source_ip: previewForm.source_ip.trim() || undefined,
    source_port: previewForm.source_port.trim()
      ? Number.parseInt(previewForm.source_port, 10) || undefined
      : undefined,
    available_nodes: splitList(previewForm.available_nodes),
  })

  const handlePreview = async () => {
    setPreviewLoading(true)
    try {
      const result = await egressIdentityPreviewMatch(buildPreviewRequest())
      setPreviewResult(result)
    } catch (error: any) {
      showNotice(
        'error',
        error?.message || error?.toString() || '预览匹配失败',
      )
    } finally {
      setPreviewLoading(false)
    }
  }

  const handleAssign = async () => {
    setAssignLoading(true)
    try {
      const result = await egressIdentityAssignMatch(buildPreviewRequest())
      setPreviewResult(result)
      await refreshAssignments()
      showNotice('success', '已创建运行时出口身份分配')
    } catch (error: any) {
      showNotice(
        'error',
        error?.message || error?.toString() || '创建分配失败',
      )
    } finally {
      setAssignLoading(false)
    }
  }

  const handleClearAssignment = async (key: string) => {
    try {
      await egressIdentityClearAssignment(key)
      await refreshAssignments()
      showNotice('success', '运行时分配已清除')
    } catch (error: any) {
      showNotice(
        'error',
        error?.message || error?.toString() || '清除分配失败',
      )
    }
  }

  const handlePreviewFormChange = (
    patch: Partial<EgressIdentityPreviewFormState>,
  ) => {
    setPreviewForm((current) => ({
      ...current,
      ...patch,
    }))
  }

  return {
    previewResult,
    previewLoading,
    assignLoading,
    assignmentsLoading,
    previewForm,
    refreshAssignments,
    handlePreview,
    handleAssign,
    handleClearAssignment,
    handlePreviewFormChange,
  }
}
