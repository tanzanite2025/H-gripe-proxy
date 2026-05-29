import { useLockFn } from 'ahooks'
import type { Ref } from 'react'
import { useImperativeHandle, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Dialog, Button, Box } from '@/components/tailwind'
import { BaseEmpty, DialogRef } from '@/components/base'
import { useClashInfo } from '@/hooks/data'
import { useVerge } from '@/hooks/system'
import { openWebUrl } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

import { WebUIItem } from './webui-item'

const DEFAULT_WEB_UI_LIST = [
  'https://metacubex.github.io/metacubexd/#/setup?http=true&hostname=%host&port=%port&secret=%secret',
  'https://yacd.metacubex.one/?hostname=%host&port=%port&secret=%secret',
  'https://board.zash.run.place/#/setup?http=true&hostname=%host&port=%port&secret=%secret',
]

export function WebUIViewer({ ref }: { ref?: Ref<DialogRef> }) {
  const { t } = useTranslation()

  const { clashInfo } = useClashInfo()
  const { verge, patchVerge, mutateVerge } = useVerge()

  const [open, setOpen] = useState(false)
  const [editing, setEditing] = useState(false)

  useImperativeHandle(ref, () => ({
    open: () => setOpen(true),
    close: () => setOpen(false),
  }))

  const webUIList = verge?.web_ui_list || DEFAULT_WEB_UI_LIST

  const webUIEntries = useMemo(() => {
    const counts: Record<string, number> = {}
    return webUIList.map((item, index) => {
      const keyBase = item && item.trim().length > 0 ? item : 'entry'
      const count = counts[keyBase] ?? 0
      counts[keyBase] = count + 1
      return {
        item,
        index,
        key: `${keyBase}-${count}`,
      }
    })
  }, [webUIList])

  const handleAdd = useLockFn(async (value: string) => {
    const newList = [...webUIList, value]
    mutateVerge((old) => (old ? { ...old, web_ui_list: newList } : old), false)
    await patchVerge({ web_ui_list: newList })
  })

  const handleChange = useLockFn(async (index: number, value?: string) => {
    const newList = [...webUIList]
    newList[index] = value ?? ''
    mutateVerge((old) => (old ? { ...old, web_ui_list: newList } : old), false)
    await patchVerge({ web_ui_list: newList })
  })

  const handleDelete = useLockFn(async (index: number) => {
    const newList = [...webUIList]
    newList.splice(index, 1)
    mutateVerge((old) => (old ? { ...old, web_ui_list: newList } : old), false)
    await patchVerge({ web_ui_list: newList })
  })

  const handleOpenUrl = useLockFn(async (value?: string) => {
    if (!value) return
    try {
      let url = value.trim().replaceAll('%host', '127.0.0.1')

      if (url.includes('%port') || url.includes('%secret')) {
        if (!clashInfo) throw new Error('failed to get clash info')
        if (!clashInfo.server?.includes(':')) {
          throw new Error(`failed to parse the server "${clashInfo.server}"`)
        }

        const port = clashInfo.server
          .slice(clashInfo.server.indexOf(':') + 1)
          .trim()

        url = url.replaceAll('%port', port || '9097')
        url = url.replaceAll(
          '%secret',
          encodeURIComponent(clashInfo.secret || ''),
        )
      }

      await openWebUrl(url)
    } catch (e: any) {
      showNotice.error(e)
    }
  })

  return (
    <Dialog
      open={open}
      onClose={() => setOpen(false)}
      title={t('settings.modals.webUI.title')}
      maxWidth="md"
      actions={
        <Button onClick={() => setOpen(false)}>
          {t('shared.actions.close')}
        </Button>
      }
    >
      <Box className="w-[450px] h-[300px] pb-4 overflow-y-auto select-text">
        <Box className="mb-4 flex justify-end">
          <Button
            variant="primary"
            size="small"
            disabled={editing}
            onClick={() => setEditing(true)}
          >
            {t('shared.actions.new')}
          </Button>
        </Box>
        {!editing && webUIList.length === 0 && (
          <BaseEmpty
            extra={
              <p className="mt-8 text-xs">
                {t('settings.modals.webUI.messages.placeholderInstruction')}
              </p>
            }
          />
        )}

        {webUIEntries.map(({ item, index, key }) => (
          <WebUIItem
            key={key}
            value={item}
            onChange={(v) => handleChange(index, v)}
            onDelete={() => handleDelete(index)}
            onOpenUrl={handleOpenUrl}
          />
        ))}
        {editing && (
          <WebUIItem
            value=""
            onlyEdit
            onChange={(v) => {
              setEditing(false)
              handleAdd(v || '')
            }}
            onCancel={() => setEditing(false)}
          />
        )}
      </Box>
    </Dialog>
  )
}
