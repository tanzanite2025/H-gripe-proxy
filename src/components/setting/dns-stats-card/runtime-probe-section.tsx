import { useLockFn } from 'ahooks'
import yaml from 'js-yaml'
import { Activity } from 'lucide-react'
import { useMemo, useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import type { DnsRuntimeStatus } from '@/services/cmds'
import { dnsControlledRuntimeProbe } from '@/services/dns-api'
import type { DnsResolverRuntimeProbeReport } from '@/services/dns-api'
import { showNotice } from '@/services/notice-service'

import { DnsChipRow, DnsSectionHeading, DnsTextRow } from './shared'

interface RuntimeProbeSectionProps {
  runtimeStatus: DnsRuntimeStatus
  report: DnsResolverRuntimeProbeReport | null
  onReportChange: (report: DnsResolverRuntimeProbeReport) => void
}

const CONTROLLED_PROBE_DOMAIN = 'example.com'

function buildRuntimeProbeYaml(runtimeStatus: DnsRuntimeStatus) {
  const nameservers = [
    ...runtimeStatus.derived.domestic_dns,
    ...runtimeStatus.derived.foreign_dns,
  ].filter(
    (server, index, servers) => server && servers.indexOf(server) === index,
  )

  return {
    nameservers,
    yaml: yaml.dump(
      {
        dns: {
          enable: true,
          nameserver: nameservers,
        },
      },
      { lineWidth: -1 },
    ),
  }
}

export function RuntimeProbeSection({
  runtimeStatus,
  report,
  onReportChange,
}: RuntimeProbeSectionProps) {
  const [pending, setPending] = useState(false)
  const { nameservers, yaml: probeYaml } = useMemo(
    () => buildRuntimeProbeYaml(runtimeStatus),
    [runtimeStatus],
  )
  const canProbe = nameservers.length > 0

  const handleProbe = useLockFn(async () => {
    if (!canProbe) {
      return
    }

    setPending(true)
    try {
      const nextReport = await dnsControlledRuntimeProbe(
        probeYaml,
        CONTROLLED_PROBE_DOMAIN,
      )
      onReportChange(nextReport)
      showNotice.success('DNS Rust 受控探测已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(false)
    }
  })

  return (
    <div>
      <div className="mb-2 flex items-center justify-between gap-3">
        <DnsSectionHeading
          title="Rust 受控探测"
          icon={<Activity className="h-3 w-3" />}
        />
        <Button
          size="small"
          variant="outlined"
          onClick={handleProbe}
          disabled={!canProbe || pending}
        >
          {pending ? '探测中...' : '探测当前 DNS'}
        </Button>
      </div>

      <div className="mb-2 text-xs text-muted-foreground">
        使用当前运行态 DNS 列表构造 planning-only YAML，只探测 Rust resolver
        可支持的 nameserver，不修改 Mihomo runtime。
      </div>

      {!canProbe && (
        <div className="rounded-md border border-border px-3 py-2 text-xs text-muted-foreground">
          当前运行态没有可用于受控探测的 DNS 服务器。
        </div>
      )}

      {report && (
        <div className="space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsTextRow
              label="探测域名"
              value={report.testDomain}
              valueClassName="text-xs font-bold"
            />
            <DnsChipRow
              label="结果"
              chipLabel={`${report.summary.healthyTargets}/${report.summary.runtimeSupportedTargets} healthy`}
              chipColor={
                report.summary.failedTargets === 0 &&
                report.summary.unsupportedTargets === 0
                  ? 'success'
                  : 'warning'
              }
            />
          </div>

          <div className="flex flex-wrap gap-2">
            {report.targets.map((target) => (
              <Chip
                key={`${target.server}-${target.protocol}`}
                size="small"
                color={target.healthy ? 'success' : 'warning'}
                label={`${target.server} · ${target.providerLabel ?? target.protocol}`}
                title={target.message}
              />
            ))}
          </div>

          {report.warnings.length > 0 && (
            <div className="rounded-md border border-warning/40 bg-warning/5 px-3 py-2 text-xs text-muted-foreground">
              {report.warnings.join('；')}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
