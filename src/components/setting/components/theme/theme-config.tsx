import { Edit } from 'lucide-react'
import { useLockFn } from 'ahooks'
import {
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
} from 'react'
import { useTranslation } from 'react-i18next'

import { Dialog, Button, TextField, Box } from '@/components/tailwind'
import { DialogRef } from '@/components/base'
import { EditorViewer } from '@/components/profile/editor-viewer'
import { useVerge } from '@/hooks/system'
import { defaultDarkTheme, defaultTheme } from '@/pages/_core/theme'
import { showNotice } from '@/services/notice-service'
import { useTheme } from '@/hooks/use-theme'

export function ThemeViewer(props: { ref?: React.Ref<DialogRef> }) {
  const { ref } = props
  const { t } = useTranslation()

  const [open, setOpen] = useState(false)
  const [editorOpen, setEditorOpen] = useState(false)
  const [cssEditorValue, setCssEditorValue] = useState('')
  const [cssEditorSavedValue, setCssEditorSavedValue] = useState('')
  const { verge, patchVerge } = useVerge()
  const { theme_setting } = verge ?? {}
  const [theme, setTheme] = useState(theme_setting || {})
  // Latest theme ref to avoid stale closures when saving CSS
  const themeRef = useRef(theme)
  useEffect(() => {
    themeRef.current = theme
  }, [theme])

  useImperativeHandle(ref, () => ({
    open: () => {
      setOpen(true)
      setTheme({ ...theme_setting })
    },
    close: () => setOpen(false),
  }))

  const handleChange = (field: keyof typeof theme) => (e: any) => {
    setTheme((t) => ({ ...t, [field]: e.target.value }))
  }

  const onSave = useLockFn(async () => {
    try {
      await patchVerge({ theme_setting: theme })
      setOpen(false)
    } catch (err) {
      showNotice.error(err)
    }
  })

  const { mode } = useTheme()

  const dt = mode === 'light' ? defaultTheme : defaultDarkTheme

  type ThemeKey = keyof typeof theme & keyof typeof defaultTheme

  const fieldDefinitions: Array<{ labelKey: string; key: ThemeKey }> = useMemo(
    () => [
      {
        labelKey: 'settings.components.verge.theme.fields.primaryColor',
        key: 'primary_color',
      },
      {
        labelKey: 'settings.components.verge.theme.fields.secondaryColor',
        key: 'secondary_color',
      },
      {
        labelKey: 'settings.components.verge.theme.fields.primaryText',
        key: 'primary_text',
      },
      {
        labelKey: 'settings.components.verge.theme.fields.secondaryText',
        key: 'secondary_text',
      },
    ],
    [],
  )

  const openCssEditor = () => {
    const nextCss = themeRef.current?.css_injection ?? ''
    setCssEditorValue(nextCss)
    setCssEditorSavedValue(nextCss)
    setEditorOpen(true)
  }

  const handleSaveCss = useLockFn(async () => {
    const prevTheme = themeRef.current || {}
    setTheme({ ...prevTheme, css_injection: cssEditorValue })
    setCssEditorSavedValue(cssEditorValue)
  })

  const renderItem = (labelKey: string, key: ThemeKey) => {
    const label = t(labelKey)
    return (
      <Box key={key} className="flex items-center py-2 px-1">
        <span className="flex-1">{label}</span>
        <div
          className="w-6 h-6 rounded-full mr-3"
          style={{ background: theme[key] || dt[key] }}
        />
        <TextField
          size="small"
          autoComplete="off"
          className="w-[135px]"
          value={theme[key] ?? ''}
          placeholder={dt[key]}
          onChange={handleChange(key)}
          onKeyDown={(e) => e.key === 'Enter' && onSave()}
        />
      </Box>
    )
  }

  return (
    <Dialog
      open={open}
      onClose={() => setOpen(false)}
      title={t('settings.components.verge.theme.title')}
      maxWidth="sm"
      actions={
        <>
          <Button onClick={() => setOpen(false)}>
            {t('shared.actions.cancel')}
          </Button>
          <Button onClick={onSave} variant="primary">
            {t('shared.actions.save')}
          </Button>
        </>
      }
    >
      <Box className="w-[400px] max-h-[505px] overflow-auto pb-0">
        <Box className="pt-0">
          {fieldDefinitions.map((field) => renderItem(field.labelKey, field.key))}

          <Box className="flex items-center py-2 px-1">
            <span className="flex-1">
              {t('settings.components.verge.theme.fields.fontFamily')}
            </span>
            <TextField
              size="small"
              autoComplete="off"
              className="w-[135px]"
              value={theme.font_family ?? ''}
              onChange={handleChange('font_family')}
              onKeyDown={(e) => e.key === 'Enter' && onSave()}
            />
          </Box>
          <Box className="flex items-center py-2 px-1">
            <span className="flex-1">
              {t('settings.components.verge.theme.fields.cssInjection')}
            </span>
            <Button
              variant="outlined"
              onClick={openCssEditor}
              className="flex items-center gap-2"
            >
              <Edit size={16} />
              {t('settings.components.verge.theme.actions.editCss')}
            </Button>
            {editorOpen && (
              <EditorViewer
                open={true}
                title={t('settings.components.verge.theme.dialogs.editCssTitle')}
                value={cssEditorValue}
                language="css"
                path="theme-css.css"
                dirty={cssEditorValue !== cssEditorSavedValue}
                onChange={setCssEditorValue}
                onSave={handleSaveCss}
                onClose={() => {
                  setEditorOpen(false)
                }}
              />
            )}
          </Box>
        </Box>
      </Box>
    </Dialog>
  )
}
