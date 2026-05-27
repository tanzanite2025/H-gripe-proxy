import { FeaturedPlayListRounded } from '@mui/icons-material'
import { useLockFn } from 'ahooks'
import { useCallback, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { EditorViewer } from '@/components/profile/editor-viewer'
import { Badge } from '@/components/tailwind/Badge'
import { IconButton } from '@/components/tailwind/IconButton'
import { Menu, MenuItem } from '@/components/tailwind/Menu'
import { useEditorDocument } from '@/hooks/ui'
import { viewProfile, readProfileFile, saveProfileFile } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

import { LogViewer } from './log-viewer'
import { ProfileBox } from './profile-box'

interface Props {
  logInfo?: [string, string][]
  id: 'Merge' | 'Script'
  onSave?: (prev?: string, curr?: string) => void
}

const EMPTY_LOG_INFO: [string, string][] = []

// profile enhanced item
export const ProfileMore = (props: Props) => {
  const { id, logInfo, onSave } = props

  const entries = logInfo ?? EMPTY_LOG_INFO
  const { t } = useTranslation()
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null)
  const [position, setPosition] = useState({ left: 0, top: 0 })
  const [fileOpen, setFileOpen] = useState(false)
  const [logOpen, setLogOpen] = useState(false)

  const loadDocument = useCallback(() => readProfileFile(id), [id])
  const document = useEditorDocument({
    open: fileOpen,
    load: loadDocument,
  })

  const onEditFile = () => {
    setAnchorEl(null)
    setFileOpen(true)
  }

  const onOpenFile = useLockFn(async () => {
    setAnchorEl(null)
    try {
      await viewProfile(id)
    } catch (err) {
      showNotice.error(err)
    }
  })

  const hasError = entries.some(([level]) => level === 'exception')

  const globalTitles: Record<Props['id'], string> = {
    Merge: 'profiles.components.more.global.merge',
    Script: 'profiles.components.more.global.script',
  }

  const chipLabels: Record<Props['id'], string> = {
    Merge: 'profiles.components.more.chips.merge',
    Script: 'profiles.components.more.chips.script',
  }

  const itemMenu = [
    { label: 'profiles.components.menu.editFile', handler: onEditFile },
    { label: 'profiles.components.menu.openFile', handler: onOpenFile },
  ]

  const handleSave = useLockFn(async () => {
    const currentValue = document.value
    if (!(await saveProfileFile(id, currentValue))) {
      await document.reload()
      return
    }
    onSave?.(document.savedValue, currentValue)
    document.markSaved(currentValue)
  })

  return (
    <>
      <ProfileBox
        onDoubleClick={onEditFile}
        onContextMenu={(event) => {
          const { clientX, clientY } = event
          setPosition({ top: clientY, left: clientX })
          setAnchorEl(event.currentTarget as HTMLElement)
          event.preventDefault()
        }}
      >
        <div className="flex justify-between items-center mb-1">
          <h2
            className="text-xl font-semibold overflow-hidden text-ellipsis whitespace-nowrap"
            title={t(globalTitles[id])}
            style={{ width: 'calc(100% - 52px)' }}
          >
            {t(globalTitles[id])}
          </h2>

          <span className="inline-block px-2 h-5 text-xs border border-primary text-primary rounded capitalize leading-5">
            {t(chipLabels[id])}
          </span>
        </div>

        <div className="h-[26px] flex items-center justify-between leading-none">
          {id === 'Script' &&
            (hasError ? (
              <Badge variant="dot" color="error">
                <IconButton
                  size="small"
                  color="error"
                  title={t('profiles.modals.logViewer.title')}
                  onClick={() => setLogOpen(true)}
                >
                  <FeaturedPlayListRounded fontSize="inherit" />
                </IconButton>
              </Badge>
            ) : (
              <IconButton
                size="small"
                title={t('profiles.modals.logViewer.title')}
                onClick={() => setLogOpen(true)}
              >
                <FeaturedPlayListRounded fontSize="inherit" />
              </IconButton>
            ))}
        </div>
      </ProfileBox>

      <Menu
        open={!!anchorEl}
        anchorEl={anchorEl}
        onClose={() => setAnchorEl(null)}
        anchorPosition={position}
        anchorReference="anchorPosition"
        onContextMenu={(e) => {
          setAnchorEl(null)
          e.preventDefault()
        }}
      >
        {itemMenu
          .filter((item: any) => item.show !== false)
          .map((item) => (
            <MenuItem
              key={item.label}
              onClick={item.handler}
              className={`min-w-[120px] ${
                item.label === 'Delete' ? 'text-red-500' : ''
              }`}
            >
              {t(item.label)}
            </MenuItem>
          ))}
      </Menu>
      {fileOpen && (
        <EditorViewer
          open={true}
          title={t(globalTitles[id])}
          value={document.value}
          language={id === 'Merge' ? 'yaml' : 'javascript'}
          path={`profile-more:${id}.${id === 'Merge' ? 'yaml' : 'js'}`}
          loading={document.loading}
          dirty={document.dirty}
          onChange={document.setValue}
          onSave={handleSave}
          onClose={() => setFileOpen(false)}
        />
      )}
      {logOpen && (
        <LogViewer
          open={logOpen}
          logInfo={entries}
          onClose={() => setLogOpen(false)}
        />
      )}
    </>
  )
}
