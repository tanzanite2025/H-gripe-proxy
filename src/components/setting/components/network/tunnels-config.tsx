import { ChevronDown, ChevronUp, Trash2 } from 'lucide-react'
import { forwardRef, useImperativeHandle, useMemo, useState, type ChangeEvent } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog } from '@/components/base'
import {
  Button,
  Divider,
  IconButton,
  List,
  ListItem,
  ListItemButton,
  ListItemText,
  MenuItem,
  Select,
  TextField,
} from '@/components/tailwind'
import type { SelectChangeEvent } from '@/components/tailwind/Select'
import { useClash } from '@/hooks/data'
import { useProxiesData } from '@/providers/app-data-context'
import { isPortInUse } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import {
  formatHostPort,
  isValidPort,
  normalizeHost,
  normalizeListenHost,
} from '@/utils/network'

interface TunnelsViewerRef {
  open: () => void
  close: () => void
}

interface TunnelEntry {
  network: string[]
  address: string
  target: string
  proxy?: string
}

export const TunnelsViewer = forwardRef<TunnelsViewerRef>((_, ref) => {
  const { t } = useTranslation()
  const { clash, mutateClash, patchClash } = useClash()

  const [open, setOpen] = useState(false)
  const [expanded, setExpanded] = useState(false)
  const [values, setValues] = useState({
    localAddr: '',
    localPort: '',
    targetAddr: '',
    targetPort: '',
    network: 'tcp+udp',
    group: '',
    proxy: '',
  })
  const [draftTunnels, setDraftTunnels] = useState<TunnelEntry[]>([])

  useImperativeHandle(ref, () => ({
    open: () => {
      setValues(() => ({
        localAddr: '',
        localPort: '',
        targetAddr: '',
        targetPort: '',
        network: 'tcp+udp',
        group: '',
        proxy: '',
      }))
      setDraftTunnels(() => clash?.tunnels ?? [])
      setOpen(true)
      // 如果没有隧道，则自动展开
      setExpanded((clash?.tunnels ?? []).length === 0)
    },
    close: () => {
      setOpen(false)
    },
  }))

  const tunnelEntries = useMemo(() => {
    const counts: Record<string, number> = {}
    return draftTunnels.map((tunnel, index) => {
      const base = `${tunnel.address}_${tunnel.target}_${tunnel.network.join('+')}`
      const occurrence = (counts[base] = (counts[base] ?? 0) + 1)
      return {
        index,
        key: `${base}_${occurrence}`,
        address: tunnel.address,
        target: tunnel.target,
        network: tunnel.network,
        proxy: tunnel.proxy,
      }
    })
  }, [draftTunnels])

  const { proxies } = useProxiesData()

  const proxyGroups = useMemo<IProxyGroupItem[]>(() => {
    return proxies?.groups ?? []
  }, [proxies])

  const groupNames = useMemo<string[]>(
    () => proxyGroups.map((group) => group.name),
    [proxyGroups],
  )

  const proxyOptions = useMemo<IProxyItem[]>(() => {
    const group = proxyGroups.find((item) => item.name === values.group)
    return group?.all ?? []
  }, [proxyGroups, values.group])

  const handleSave = async () => {
    try {
      await patchClash({ tunnels: draftTunnels })
      await mutateClash()
      showNotice.success('shared.feedback.notifications.common.saveSuccess')
      setOpen(false)
    } catch (err: any) {
      showNotice.error(err)
    }
  }

  const handleAdd = async () => {
    const { localAddr, localPort, targetAddr, targetPort, network, proxy } =
      values

    // 基础非空校验
    if (!localAddr || !localPort || !targetAddr || !targetPort) {
      showNotice.error(
        'settings.sections.clash.form.fields.tunnels.messages.incomplete',
      )
      return
    }

    // 本地地址校验（host）
    const localHost = normalizeListenHost(localAddr)
    if (!localHost) {
      showNotice.error(
        'settings.sections.clash.form.fields.tunnels.messages.invalidLocalAddr',
      )
      return
    }

    // 本地端口校验 (port)
    if (!isValidPort(localPort)) {
      showNotice.error(
        'settings.sections.clash.form.fields.tunnels.messages.invalidLocalPort',
      )
      return
    }
    const inUse = await isPortInUse(Number(localPort))
    if (inUse) {
      showNotice.error('settings.modals.clashPort.messages.portInUse', {
        port: localPort,
      })
      return
    }

    // 目标地址校验 (host)
    const targetHost = normalizeHost(targetAddr)
    if (!targetHost) {
      showNotice.error(
        'settings.sections.clash.form.fields.tunnels.messages.invalidTargetAddr',
      )
      return
    }

    // 目标端口校验 (port)
    if (!isValidPort(targetPort)) {
      showNotice.error(
        'settings.sections.clash.form.fields.tunnels.messages.invalidTargetPort',
      )
      return
    }

    // 构造新 entry
    const entry: TunnelEntry = {
      network: network === 'tcp+udp' ? ['tcp', 'udp'] : [network],
      address: formatHostPort(localHost, localPort),
      target: formatHostPort(targetHost, targetPort),
      ...(proxy ? { proxy } : {}),
    }

    // 写入配置 + 清空输入
    setDraftTunnels((prev) => [...prev, entry])

    setValues((v) => ({
      ...v,
      localAddr: '',
      localPort: '',
      targetAddr: '',
      targetPort: '',
      network: 'tcp+udp',
    }))
  }

  const handleDelete = (index: number) => {
    setDraftTunnels((prev) => prev.filter((_, i) => i !== index))
  }

  return (
    <BaseDialog
      open={open}
      title={t('settings.sections.clash.form.fields.tunnels.title')}
      panelStyle={{ width: 450 }}
      okBtn={t('shared.actions.save')}
      cancelBtn={t('shared.actions.cancel')}
      onClose={() => {
        setOpen(false)
      }}
      onCancel={() => {
        setOpen(false)
      }}
      onOk={handleSave}
    >
      <List>
        {draftTunnels.length > 0 && (
          <>
            <ListItem className="py-1 px-0 opacity-60">
              <ListItemText
                primary={t(
                  'settings.sections.clash.form.fields.tunnels.existing',
                )}
              />
            </ListItem>
            <List component="nav">
              {tunnelEntries.map((item) => (
                <ListItem
                  key={`${item.key}`}
                  className="py-1 px-0 justify-between gap-2"
                >
                  <ListItemText
                    primary={`${item.address} → ${item.target}`}
                    secondary={`${item.network.join(', ')} · ${
                      item.proxy ??
                      t('settings.sections.clash.form.fields.tunnels.default')
                    }`}
                  />
                  <IconButton
                    size="small"
                    color="error"
                    onClick={() => handleDelete(item.index)}
                  >
                    <Trash2 className="h-4 w-4" />
                  </IconButton>
                </ListItem>
              ))}
            </List>
            <Divider className="my-8" />
          </>
        )}
        <ListItemButton
          className="py-1 px-0 opacity-80"
          onClick={() => setExpanded((v) => !v)}
        >
          <ListItemText
            primary={t(
              'settings.sections.clash.form.fields.tunnels.actions.addNew',
            )}
          />
          {expanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
        </ListItemButton>
        {expanded && (
          <ListItem className="py-2 px-0">
            <div style={{ width: '100%' }}>
              {/* 输入框区域 */}
              {/* 协议 */}
              <ListItem className="py-[6px] px-[2px]">
                <ListItemText
                  primary={t(
                    'settings.sections.clash.form.fields.tunnels.protocols',
                  )}
                />
                <Select
                  size="small"
                  className="w-[200px]"
                  value={values.network}
                  onChange={(e: SelectChangeEvent) =>
                    setValues((v) => ({
                      ...v,
                      network: e.target.value as string,
                    }))
                  }
                >
                  <MenuItem value="tcp">TCP</MenuItem>
                  <MenuItem value="udp">UDP</MenuItem>
                  <MenuItem value="tcp+udp">TCP + UDP</MenuItem>
                </Select>
              </ListItem>

              {/* 本地监听地址 */}
              <ListItem className="py-[6px] px-[2px]">
                <ListItemText
                  primary={t(
                    'settings.sections.clash.form.fields.tunnels.localAddr',
                  )}
                />
                <TextField
                  autoComplete="new-password"
                  size="small"
                  className="w-[200px]"
                  value={values.localAddr}
                  placeholder="127.0.0.1"
                  onChange={(e: ChangeEvent<HTMLInputElement>) =>
                    setValues((v) => ({ ...v, localAddr: e.target.value }))
                  }
                />
              </ListItem>

              {/* 本地监听端口 */}
              <ListItem className="py-[6px] px-[2px]">
                <ListItemText
                  primary={t(
                    'settings.sections.clash.form.fields.tunnels.localPort',
                  )}
                />
                <TextField
                  autoComplete="new-password"
                  size="small"
                  type="number"
                  className="w-[200px]"
                  value={values.localPort}
                  placeholder="6553"
                  onChange={(e: ChangeEvent<HTMLInputElement>) =>
                    setValues((v) => ({ ...v, localPort: e.target.value }))
                  }
                />
              </ListItem>

              {/* 目标服务器地址 */}
              <ListItem className="py-[6px] px-[2px]">
                <ListItemText
                  primary={t(
                    'settings.sections.clash.form.fields.tunnels.targetAddr',
                  )}
                />
                <TextField
                  autoComplete="new-password"
                  size="small"
                  className="w-[200px]"
                  value={values.targetAddr}
                  placeholder="8.8.8.8"
                  onChange={(e: ChangeEvent<HTMLInputElement>) =>
                    setValues((v) => ({ ...v, targetAddr: e.target.value }))
                  }
                />
              </ListItem>

              {/* 目标服务器端口 */}
              <ListItem className="py-[6px] px-[2px]">
                <ListItemText
                  primary={t(
                    'settings.sections.clash.form.fields.tunnels.targetPort',
                  )}
                />
                <TextField
                  autoComplete="new-password"
                  size="small"
                  type="number"
                  className="w-[200px]"
                  value={values.targetPort}
                  placeholder="53"
                  onChange={(e: ChangeEvent<HTMLInputElement>) =>
                    setValues((v) => ({ ...v, targetPort: e.target.value }))
                  }
                />
              </ListItem>

              {/* 代理组 */}
              <ListItem className="py-[6px] px-[2px]">
                <ListItemText
                  primary={
                    <>
                      {t(
                        'settings.sections.clash.form.fields.tunnels.proxyGroup',
                      )}
                      <span style={{ fontSize: '0.9rem', color: 'gray' }}>
                        {' '}
                        (
                        {t(
                          'settings.sections.clash.form.fields.tunnels.optional',
                        )}
                        )
                      </span>
                    </>
                  }
                />
                <Select
                  size="small"
                  className="w-[200px]"
                  value={values.group}
                  onChange={(e: SelectChangeEvent) => {
                    const nextGroup = e.target.value as string
                    const group = proxyGroups.find((g) => g.name === nextGroup)
                    const firstProxy = group?.all?.[0].name ?? ''

                    setValues((v) => ({
                      ...v,
                      group: nextGroup,
                      proxy: firstProxy, // 组切换时自动选第一条节点
                    }))
                  }}
                >
                  <MenuItem value="">
                    {t('settings.sections.clash.form.fields.tunnels.default')}
                  </MenuItem>
                  {groupNames.map((name) => (
                    <MenuItem key={name} value={name}>
                      {name}
                    </MenuItem>
                  ))}
                </Select>
              </ListItem>

              {/* 代理节点 */}
              <ListItem className="py-[6px] px-[2px]">
                <ListItemText
                  primary={
                    <>
                      {t(
                        'settings.sections.clash.form.fields.tunnels.proxyNode',
                      )}
                      <span style={{ fontSize: '0.9rem', color: 'gray' }}>
                        {' '}
                        (
                        {t(
                          'settings.sections.clash.form.fields.tunnels.optional',
                        )}
                        )
                      </span>
                    </>
                  }
                />
                <Select
                  size="small"
                  className="w-[200px]"
                  value={values.proxy}
                  onChange={(e: SelectChangeEvent) =>
                    setValues((v) => ({
                      ...v,
                      proxy: e.target.value as string,
                    }))
                  }
                  disabled={!values.group} // 没选组就禁用
                >
                  <MenuItem value="">
                    {t('settings.sections.clash.form.fields.tunnels.default')}
                  </MenuItem>
                  {proxyOptions.map((node) => (
                    <MenuItem key={node.name} value={node.name}>
                      {node.name}
                    </MenuItem>
                  ))}
                </Select>
              </ListItem>

              {/* 添加按钮 */}
              <Button
                variant="contained"
                size="small"
                className="mt-[6px] mr-[2px] ml-auto block"
                color="success"
                onClick={handleAdd}
              >
                {t('settings.sections.clash.form.fields.tunnels.actions.add')}
              </Button>
            </div>
          </ListItem>
        )}
      </List>
    </BaseDialog>
  )
})
