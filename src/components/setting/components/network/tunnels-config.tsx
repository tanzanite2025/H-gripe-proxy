import { ChevronDown, ChevronUp, Trash2 } from 'lucide-react'
import {
  forwardRef,
  useImperativeHandle,
  useMemo,
  useState,
  type ChangeEvent,
} from 'react'
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
  Select,
  SelectMenuItem,
  TextField,
} from '@/components/tailwind'
import type { SelectChangeEvent } from '@/components/tailwind/Select'
import { useClash } from '@/hooks/data'
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
}

const createEmptyValues = () => ({
  localAddr: '',
  localPort: '',
  targetAddr: '',
  targetPort: '',
  network: 'tcp+udp',
})

const sanitizeTunnels = (
  tunnels: Array<TunnelEntry & { proxy?: string }> = [],
): TunnelEntry[] =>
  tunnels.map(({ network, address, target }) => ({
    network,
    address,
    target,
  }))

export const TunnelsViewer = forwardRef<TunnelsViewerRef>((_, ref) => {
  const { t } = useTranslation()
  const { clash, mutateClash, patchClash } = useClash()

  const [open, setOpen] = useState(false)
  const [expanded, setExpanded] = useState(false)
  const [values, setValues] = useState(createEmptyValues)
  const [draftTunnels, setDraftTunnels] = useState<TunnelEntry[]>([])

  useImperativeHandle(ref, () => ({
    open: () => {
      setValues(createEmptyValues)
      setDraftTunnels(() => sanitizeTunnels(clash?.tunnels ?? []))
      setOpen(true)
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
      }
    })
  }, [draftTunnels])

  const handleSave = async () => {
    try {
      const tunnels = sanitizeTunnels(draftTunnels)
      await patchClash({ tunnels })
      await mutateClash()
      setDraftTunnels(tunnels)
      showNotice.success('shared.feedback.notifications.common.saveSuccess')
      setOpen(false)
    } catch (err: any) {
      showNotice.error(err)
    }
  }

  const handleAdd = async () => {
    const { localAddr, localPort, targetAddr, targetPort, network } = values

    if (!localAddr || !localPort || !targetAddr || !targetPort) {
      showNotice.error(
        'settings.sections.clash.form.fields.tunnels.messages.incomplete',
      )
      return
    }

    const localHost = normalizeListenHost(localAddr)
    if (!localHost) {
      showNotice.error(
        'settings.sections.clash.form.fields.tunnels.messages.invalidLocalAddr',
      )
      return
    }

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

    const targetHost = normalizeHost(targetAddr)
    if (!targetHost) {
      showNotice.error(
        'settings.sections.clash.form.fields.tunnels.messages.invalidTargetAddr',
      )
      return
    }

    if (!isValidPort(targetPort)) {
      showNotice.error(
        'settings.sections.clash.form.fields.tunnels.messages.invalidTargetPort',
      )
      return
    }

    const entry: TunnelEntry = {
      network: network === 'tcp+udp' ? ['tcp', 'udp'] : [network],
      address: formatHostPort(localHost, localPort),
      target: formatHostPort(targetHost, targetPort),
    }

    setDraftTunnels((prev) => [...prev, entry])
    setValues((current) => ({
      ...current,
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
      panelStyle={{ width: 650 }}
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
                  key={item.key}
                  className="py-1 px-0 justify-between gap-2"
                >
                  <ListItemText
                    primary={`${item.address} -> ${item.target}`}
                    secondary={item.network.join(', ')}
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
          onClick={() => setExpanded((current) => !current)}
        >
          <ListItemText
            primary={t(
              'settings.sections.clash.form.fields.tunnels.actions.addNew',
            )}
          />
          {expanded ? (
            <ChevronUp className="h-4 w-4" />
          ) : (
            <ChevronDown className="h-4 w-4" />
          )}
        </ListItemButton>

        {expanded && (
          <ListItem className="py-2 px-0">
            <div style={{ width: '100%' }}>
              <ListItem className="py-[6px] px-[2px]">
                <ListItemText
                  className="flex-none w-[120px] mr-2"
                  primary={t(
                    'settings.sections.clash.form.fields.tunnels.protocols',
                  )}
                />
                <Select
                  size="small"
                  className="w-[300px]"
                  value={values.network}
                  onChange={(e: SelectChangeEvent) =>
                    setValues((current) => ({
                      ...current,
                      network: e.target.value as string,
                    }))
                  }
                >
                  <SelectMenuItem value="tcp">TCP</SelectMenuItem>
                  <SelectMenuItem value="udp">UDP</SelectMenuItem>
                  <SelectMenuItem value="tcp+udp">TCP + UDP</SelectMenuItem>
                </Select>
              </ListItem>

              <ListItem className="py-[6px] px-[2px]">
                <ListItemText
                  className="flex-none w-[120px] mr-2"
                  primary={t(
                    'settings.sections.clash.form.fields.tunnels.localAddr',
                  )}
                />
                <TextField
                  autoComplete="new-password"
                  size="small"
                  className="w-[300px]"
                  value={values.localAddr}
                  placeholder="127.0.0.1"
                  onChange={(e: ChangeEvent<HTMLInputElement>) =>
                    setValues((current) => ({
                      ...current,
                      localAddr: e.target.value,
                    }))
                  }
                />
              </ListItem>

              <ListItem className="py-[6px] px-[2px]">
                <ListItemText
                  className="flex-none w-[120px] mr-2"
                  primary={t(
                    'settings.sections.clash.form.fields.tunnels.localPort',
                  )}
                />
                <TextField
                  autoComplete="new-password"
                  size="small"
                  type="number"
                  className="w-[300px]"
                  value={values.localPort}
                  placeholder="6553"
                  onChange={(e: ChangeEvent<HTMLInputElement>) =>
                    setValues((current) => ({
                      ...current,
                      localPort: e.target.value,
                    }))
                  }
                />
              </ListItem>

              <ListItem className="py-[6px] px-[2px]">
                <ListItemText
                  className="flex-none w-[120px] mr-2"
                  primary={t(
                    'settings.sections.clash.form.fields.tunnels.targetAddr',
                  )}
                />
                <TextField
                  autoComplete="new-password"
                  size="small"
                  className="w-[300px]"
                  value={values.targetAddr}
                  placeholder="8.8.8.8"
                  onChange={(e: ChangeEvent<HTMLInputElement>) =>
                    setValues((current) => ({
                      ...current,
                      targetAddr: e.target.value,
                    }))
                  }
                />
              </ListItem>

              <ListItem className="py-[6px] px-[2px]">
                <ListItemText
                  className="flex-none w-[120px] mr-2"
                  primary={t(
                    'settings.sections.clash.form.fields.tunnels.targetPort',
                  )}
                />
                <TextField
                  autoComplete="new-password"
                  size="small"
                  type="number"
                  className="w-[300px]"
                  value={values.targetPort}
                  placeholder="53"
                  onChange={(e: ChangeEvent<HTMLInputElement>) =>
                    setValues((current) => ({
                      ...current,
                      targetPort: e.target.value,
                    }))
                  }
                />
              </ListItem>

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
