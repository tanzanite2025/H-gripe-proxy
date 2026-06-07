import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import type {
  IdentityConsistencyDriftReport,
  IdentityConsistencyReport,
  IdentityConsistencySnapshot,
} from '@/services/cmds/diagnostics'

import {
  consistencyBadgeColor,
  consistencyLevelText,
  consistencyScoreColor,
  driftKindText,
  driftValue,
  formatConsistencyValue,
  formatProxyChain,
  formatSnapshotTime,
  snapshotSummary,
} from './shared'

interface IpReputationConsistencyCardProps {
  report?: IdentityConsistencyReport
  history: IdentityConsistencySnapshot[]
  driftReport?: IdentityConsistencyDriftReport
  isRefreshing: boolean
  hasError: boolean
  onRefresh: () => void | Promise<void>
}

export function IpReputationConsistencyCard({
  report,
  history,
  driftReport,
  isRefreshing,
  hasError,
  onRefresh,
}: IpReputationConsistencyCardProps) {
  return (
    <Card>
      <div className="space-y-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="text-sm font-semibold">当前出口一致性</h3>
            <p className="mt-1 text-xs text-gray-500">
              汇总出口 IP、节点链路、DNS、TLS 指纹和 IP 风险，用于判断当前节点身份是否一致。
            </p>
          </div>
          <Button
            onClick={() => void onRefresh()}
            variant="outlined"
            size="sm"
            disabled={isRefreshing}
          >
            {isRefreshing ? '刷新中...' : '刷新'}
          </Button>
        </div>

        {hasError ? (
          <div className="rounded-lg border border-red-200 bg-red-50 p-3 text-xs text-red-600 dark:border-red-900/40 dark:bg-red-950/20">
            一致性报告获取失败
          </div>
        ) : report ? (
          <>
            <div className="grid grid-cols-1 gap-4 md:grid-cols-4">
              <div>
                <p className="text-xs text-gray-500">一致性评分</p>
                <p
                  className={`text-2xl font-bold ${consistencyScoreColor[report.level]}`}
                >
                  {report.score}
                </p>
                <span
                  className={`mt-1 inline-block rounded px-2 py-0.5 text-xs font-medium ${consistencyBadgeColor[report.level]}`}
                >
                  {consistencyLevelText[report.level]}
                </span>
              </div>
              <div>
                <p className="text-xs text-gray-500">公网出口</p>
                <p className="text-sm font-medium font-mono">
                  {formatConsistencyValue(report.public_egress_ip)}
                </p>
                <p className="text-xs text-gray-400">
                  {formatConsistencyValue(report.egress_source)}
                  {report.egress_confidence !== null
                    ? ` / ${report.egress_confidence}`
                    : ''}
                </p>
              </div>
              <div>
                <p className="text-xs text-gray-500">IP 类型</p>
                <p className="text-sm font-medium">
                  {formatConsistencyValue(report.ip_type)}
                </p>
                <p className="text-xs text-gray-400">
                  {formatConsistencyValue(report.residential_state)}
                </p>
              </div>
              <div>
                <p className="text-xs text-gray-500">DNS / TLS</p>
                <p className="text-sm">
                  {formatConsistencyValue(report.dns_assessment)}
                </p>
                <p className="text-xs text-gray-400">
                  {formatConsistencyValue(report.tls_fingerprint)}
                </p>
              </div>
            </div>

            <div className="grid grid-cols-1 gap-4 border-t border-gray-200 pt-3 dark:border-gray-700 md:grid-cols-2">
              <div>
                <p className="text-xs text-gray-500">节点链路</p>
                <p className="break-all text-sm font-medium">
                  {formatProxyChain(report)}
                </p>
              </div>
              <div>
                <p className="text-xs text-gray-500">主要问题</p>
                {report.issues.length > 0 ? (
                  <div className="mt-1 space-y-1">
                    {report.issues.slice(0, 4).map((issue) => (
                      <div
                        key={`${issue.kind}-${issue.message}`}
                        className="flex items-start gap-2 text-xs"
                      >
                        <span
                          className={`mt-0.5 h-2 w-2 shrink-0 rounded-full ${
                            issue.severity === 'danger'
                              ? 'bg-red-500'
                              : issue.severity === 'warning'
                                ? 'bg-yellow-500'
                                : 'bg-gray-400'
                          }`}
                        />
                        <span className="text-gray-600 dark:text-gray-300">
                          {issue.message}
                        </span>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-green-600">暂无一致性问题</p>
                )}
              </div>
            </div>

            {driftReport && (
              <div className="border-t border-gray-200 pt-3 dark:border-gray-700">
                <div className="flex items-center justify-between gap-3">
                  <p className="text-xs text-gray-500">身份漂移</p>
                  <span
                    className={
                      driftReport.stable
                        ? 'text-xs text-green-600'
                        : 'text-xs text-yellow-600'
                    }
                  >
                    {driftReport.stable
                      ? '最近快照稳定'
                      : `检测到 ${driftReport.drift_count} 项变化`}
                  </span>
                </div>
                {!driftReport.stable && (
                  <div className="mt-2 space-y-1">
                    {driftReport.drifts.slice(0, 4).map((drift) => (
                      <div
                        key={`${drift.kind}-${drift.from || 'none'}-${drift.to || 'none'}`}
                        className="rounded bg-yellow-50 px-2 py-1.5 text-xs text-yellow-800 dark:bg-yellow-950/20 dark:text-yellow-300"
                      >
                        <span className="font-medium">
                          {driftKindText[drift.kind]}
                        </span>
                        <span className="mx-1">{driftValue(drift.from)}</span>
                        <span>{'->'}</span>
                        <span className="mx-1">{driftValue(drift.to)}</span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}

            {history.length > 0 && (
              <div className="border-t border-gray-200 pt-3 dark:border-gray-700">
                <p className="text-xs text-gray-500">最近快照</p>
                <div className="mt-2 space-y-1">
                  {history.slice(0, 3).map((snapshot) => (
                    <div
                      key={`${snapshot.observed_at}-${snapshot.report.public_egress_ip || 'unknown'}`}
                      className="flex items-center justify-between gap-3 rounded bg-gray-50 px-2 py-1.5 text-xs dark:bg-gray-900/30"
                    >
                      <span className="font-mono text-gray-500">
                        {formatSnapshotTime(snapshot)}
                      </span>
                      <span className="truncate text-gray-600 dark:text-gray-300">
                        {snapshotSummary(snapshot)}
                      </span>
                      <span
                        className={consistencyScoreColor[snapshot.report.level]}
                      >
                        {snapshot.report.score}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </>
        ) : (
          <div className="rounded-lg border border-gray-200 bg-gray-50 p-3 text-xs text-gray-500 dark:border-gray-800 dark:bg-gray-900/30">
            一致性报告加载中...
          </div>
        )}
      </div>
    </Card>
  )
}
