import { type ChangeEvent } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { TextField } from '@/components/tailwind/TextField'
import type { CoordinatorResolvedEgressIdentity } from '@/services/coordinator'
import type { ResolvedEgressIdentity } from '@/services/egress-identity'

import type { EgressIdentityPreviewFormState } from './shared'

interface Props {
  enabled: boolean
  previewForm: EgressIdentityPreviewFormState
  previewResult: ResolvedEgressIdentity | null
  profileNameMap: Record<string, string>
  domainPatternAssignments: CoordinatorResolvedEgressIdentity[]
  regularAssignments: CoordinatorResolvedEgressIdentity[]
  previewLoading: boolean
  assignLoading: boolean
  assignmentsLoading: boolean
  onPreviewFormChange: (
    patch: Partial<EgressIdentityPreviewFormState>,
  ) => void
  onRefreshAssignments: () => void | Promise<void>
  onPreview: () => void | Promise<void>
  onAssign: () => void | Promise<void>
  onClearAssignment: (key: string) => void | Promise<void>
}

export function EgressIdentityRuntimeToolsCard({
  enabled,
  previewForm,
  previewResult,
  profileNameMap,
  domainPatternAssignments,
  regularAssignments,
  previewLoading,
  assignLoading,
  assignmentsLoading,
  onPreviewFormChange,
  onRefreshAssignments,
  onPreview,
  onAssign,
  onClearAssignment,
}: Props) {
  const updateField =
    (key: keyof EgressIdentityPreviewFormState) =>
    (event: ChangeEvent<HTMLInputElement>) => {
      onPreviewFormChange({
        [key]: event.target.value,
      } as Partial<EgressIdentityPreviewFormState>)
    }

  return (
    <Card variant="outlined">
      <div className="space-y-4 p-4">
        <div className="flex items-center justify-between gap-4">
          <div>
            <p className="font-semibold">运行时诊断</p>
            <p className="mt-1 text-sm text-gray-500">
              这里可以直接预览或创建运行时 assignment，并查看当前活跃的出口身份分配。
            </p>
          </div>
          <Button
            size="small"
            variant="outlined"
            onClick={onRefreshAssignments}
            loading={assignmentsLoading}
          >
            刷新 assignment
          </Button>
        </div>

        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
          <TextField
            label="域名"
            value={previewForm.domain}
            onChange={updateField('domain')}
            fullWidth
          />
          <TextField
            label="快捷方式 ID"
            value={previewForm.shortcut_id}
            onChange={updateField('shortcut_id')}
            fullWidth
          />
          <TextField
            label="进程名"
            value={previewForm.process_name}
            onChange={updateField('process_name')}
            fullWidth
          />
          <TextField
            label="可执行路径"
            value={previewForm.exe_path}
            onChange={updateField('exe_path')}
            fullWidth
          />
          <TextField
            label="源 IP"
            value={previewForm.source_ip}
            onChange={updateField('source_ip')}
            fullWidth
          />
          <TextField
            label="源端口"
            value={previewForm.source_port}
            onChange={updateField('source_port')}
            fullWidth
          />
        </div>

        <TextField
          label="可用节点"
          value={previewForm.available_nodes}
          onChange={updateField('available_nodes')}
          helperText="用逗号或换行分隔，例如 hk-01, us-02"
          multiline
          rows={3}
          fullWidth
        />

        <div className="flex flex-wrap gap-2">
          <Button
            size="small"
            variant="outlined"
            onClick={onPreview}
            loading={previewLoading}
            disabled={!enabled}
          >
            预览匹配
          </Button>
          <Button
            size="small"
            variant="primary"
            onClick={onAssign}
            loading={assignLoading}
            disabled={!enabled}
          >
            创建 assignment
          </Button>
        </div>

        {!enabled && (
          <div className="rounded-lg bg-yellow-500 p-3 text-sm text-white">
            请先启用出口身份管理，再进行运行时预览或创建 assignment。
          </div>
        )}

        {previewResult && (
          <div className="rounded-lg bg-green-500 p-3 text-sm text-white">
            <div>
              画像：
              {profileNameMap[previewResult.profileId] ||
                previewResult.profileId}
            </div>
            <div>节点：{previewResult.selectedNode}</div>
            <div>DNS：{previewResult.dnsMode}</div>
            <div>匹配来源：{previewResult.matchedBy}</div>
            <div>
              Assignment Key：
              {previewResult.assignmentKey || '预览模式'}
            </div>
          </div>
        )}

        <div className="space-y-4">
          <div className="space-y-3 rounded-lg border border-purple-200 bg-purple-50/60 p-4 dark:border-purple-800 dark:bg-purple-950/20">
            <div>
              <div className="font-medium">
                稳定出口回写（domain-pattern）
              </div>
              <div className="mt-1 text-sm text-gray-500">
                这里展示稳定组手动选择回写到 `egress_identity`
                后形成的域名模式级运行时状态。
              </div>
            </div>

            {domainPatternAssignments.length === 0 ? (
              <div className="py-4 text-center text-sm text-gray-500">
                暂无 domain-pattern 回写 assignment
              </div>
            ) : (
              domainPatternAssignments.map((assignment) => (
                <div
                  key={`${
                    assignment.assignmentKey || assignment.profileId
                  }-${assignment.selectedNode}`}
                  className="flex items-center justify-between gap-4 rounded-lg border border-purple-200 bg-card p-3 dark:border-purple-800"
                >
                  <div>
                    <div className="font-medium">
                      {profileNameMap[assignment.profileId] ||
                        assignment.profileId}
                    </div>
                    <div className="mt-1 text-sm text-gray-500">
                      {assignment.assignmentKey || '无 assignment key'} ·{' '}
                      {assignment.matchedBy}
                    </div>
                    {assignment.sourceGroupName && (
                      <div className="mt-1 text-xs text-purple-600">
                        来源稳定组：{assignment.sourceGroupName}
                      </div>
                    )}
                    <div className="mt-2 grid grid-cols-1 gap-2 text-xs md:grid-cols-2">
                      <div className="rounded border border-purple-200 bg-purple-50/60 px-2 py-1 dark:border-purple-800 dark:bg-purple-950/20">
                        <span className="text-gray-500">
                          来源组当前选中节点：
                        </span>
                        <span className="ml-1 font-medium text-purple-700 dark:text-purple-300">
                          {assignment.sourceGroupSelectedNode || '未知'}
                        </span>
                      </div>
                      <div className="rounded border border-blue-200 bg-blue-50/60 px-2 py-1 dark:border-blue-800 dark:bg-blue-950/20">
                        <span className="text-gray-500">回写节点：</span>
                        <span className="ml-1 font-medium text-blue-700 dark:text-blue-300">
                          {assignment.selectedNode}
                        </span>
                      </div>
                    </div>
                  </div>
                  <Button
                    size="small"
                    variant="outlined"
                    disabled={!assignment.assignmentKey}
                    onClick={() =>
                      assignment.assignmentKey &&
                      onClearAssignment(assignment.assignmentKey)
                    }
                  >
                    清除
                  </Button>
                </div>
              ))
            )}
          </div>

          <div className="space-y-3 rounded-lg border border-gray-200 p-4 dark:border-gray-700">
            <div>
              <div className="font-medium">普通运行时 assignment</div>
              <div className="mt-1 text-sm text-gray-500">
                这里展示由应用、快捷方式、连接上下文等直接生成的常规运行时 assignment。
              </div>
            </div>

            {regularAssignments.length === 0 ? (
              <div className="py-4 text-center text-sm text-gray-500">
                暂无普通运行时 assignment
              </div>
            ) : (
              regularAssignments.map((assignment) => (
                <div
                  key={`${
                    assignment.assignmentKey || assignment.profileId
                  }-${assignment.selectedNode}`}
                  className="flex items-center justify-between gap-4 rounded-lg border border-gray-200 p-3 dark:border-gray-700"
                >
                  <div>
                    <div className="font-medium">
                      {profileNameMap[assignment.profileId] ||
                        assignment.profileId}
                    </div>
                    <div className="mt-1 text-sm text-gray-500">
                      {assignment.assignmentKey || '无 assignment key'} ·{' '}
                      {assignment.selectedNode} · {assignment.matchedBy}
                    </div>
                  </div>
                  <Button
                    size="small"
                    variant="outlined"
                    disabled={!assignment.assignmentKey}
                    onClick={() =>
                      assignment.assignmentKey &&
                      onClearAssignment(assignment.assignmentKey)
                    }
                  >
                    清除
                  </Button>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </Card>
  )
}
