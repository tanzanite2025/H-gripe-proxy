/**
 * DNS 配置组件（重构版）
 * 主组件负责组合所有子组件和 hooks
 */

import { useLockFn } from 'ahooks'
import yaml from 'js-yaml'
import { RefreshCw } from 'lucide-react'
import type { Ref } from 'react'
import { useCallback, useEffect, useImperativeHandle, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog, DialogRef, MonacoEditor } from '@/components/base'
import { Box, Button, List, Typography } from '@/components/tailwind'
import { showNotice } from '@/services/notice-service'
import { useThemeMode } from '@/services/states'
import type { MonacoEditorInstance } from '@/types/monaco'
import { debugLog } from '@/utils/misc'
import getSystem from '@/utils/misc'

import { DnsFallbackFields } from './components/dns-fallback-fields'
import { DnsGeneralFields } from './components/dns-general-fields'
import { DnsHostsFields } from './components/dns-hosts-fields'
import { DnsNameserverFields } from './components/dns-nameserver-fields'
import { useDnsConfig } from './hooks/use-dns-config'
import { useDnsForm } from './hooks/use-dns-form'
import { formValuesToConfig } from './utils/dns-helpers'

export function DnsViewer({ ref }: { ref?: Ref<DialogRef> }) {
  const { t } = useTranslation()
  const themeMode = useThemeMode()
  const [open, setOpen] = useState(false)
  const editorRef = useRef<MonacoEditorInstance | null>(null)

  // 配置管理
  const { loadConfig, saveConfig } = useDnsConfig()

  // 表单管理
  const {
    values,
    setValues,
    yamlContent,
    setYamlContent,
    visualization,
    setVisualization,
    updateValuesFromConfig,
    resetToDefaults,
  } = useDnsForm()

  // 初始化 DNS 配置
  const initDnsConfig = useCallback(async () => {
    try {
      const config = await loadConfig()

      if (config) {
        updateValuesFromConfig(config)
        setYamlContent(yaml.dump(config, { forceQuotes: true }))
      } else {
        resetToDefaults()
      }
    } catch (err) {
      console.error('Failed to initialize DNS config', err)
      resetToDefaults()
    }
  }, [loadConfig, updateValuesFromConfig, setYamlContent, resetToDefaults])

  useImperativeHandle(
    ref,
    () => ({
      open: () => {
        setOpen(true)
        void initDnsConfig()
      },
      close: () => setOpen(false),
    }),
    [initDnsConfig],
  )

  // 处理保存操作
  const onSave = useLockFn(async () => {
    try {
      let config: Record<string, any>

      if (visualization) {
        // 使用表单值生成配置
        config = formValuesToConfig(values)
      } else {
        // 使用YAML编辑器的值
        const parsedConfig = yaml.load(yamlContent)
        if (typeof parsedConfig !== 'object' || parsedConfig === null) {
          throw new Error(t('settings.modals.dns.errors.invalid'))
        }
        config = parsedConfig as Record<string, any>
      }

      // 保存配置
      const success = await saveConfig(config)
      if (success) {
        setOpen(false)
        showNotice.success('settings.modals.dns.messages.saved')
      }
    } catch (err) {
      showNotice.error(err)
    }
  })

  // YAML编辑器内容变更处理
  const handleYamlChange = (value?: string) => {
    setYamlContent(value || '')

    // 允许YAML编辑后立即分析和更新表单值
    try {
      const config = yaml.load(value || '') as any
      if (config && typeof config === 'object') {
        setTimeout(() => {
          updateValuesFromConfig(config)
        }, 300)
      }
    } catch (err) {
      debugLog('YAML解析错误，忽略自动更新', err)
    }
  }

  // 处理表单值变化
  const handleChange = (field: string) => (event: any) => {
    const value =
      event.target.type === 'checkbox'
        ? event.target.checked
        : event.target.value

    setValues((prev) => ({
      ...prev,
      [field]: value,
    }))
  }

  // 清理编辑器
  useEffect(() => {
    return () => {
      editorRef.current?.dispose()
      editorRef.current = null
    }
  }, [])

  return (
    <BaseDialog
      open={open}
      disableEnforceFocus={!visualization}
      title={
        <Box className="flex justify-between items-center">
          <span className="uds-title-h2">
            {t('settings.modals.dns.dialog.title')}
          </span>
          <Box className="flex items-center gap-4">
            <Button
              variant="outlined"
              size="small"
              color="warning"
              startIcon={<RefreshCw className="h-4 w-4" />}
              onClick={resetToDefaults}
            >
              {t('shared.actions.resetToDefault')}
            </Button>
            <Button
              variant="primary"
              size="small"
              onClick={() => {
                setVisualization((prev) => !prev)
              }}
            >
              {visualization
                ? t('shared.editorModes.advanced')
                : t('shared.editorModes.visualization')}
            </Button>
          </Box>
        </Box>
      }
      contentSx={{
        width: 550,
        overflow: 'auto',
        ...(visualization
          ? {}
          : { padding: '0 24px', display: 'flex', flexDirection: 'column' }),
      }}
      okBtn={t('shared.actions.save')}
      cancelBtn={t('shared.actions.cancel')}
      onClose={() => setOpen(false)}
      onCancel={() => setOpen(false)}
      onOk={onSave}
    >
      {/* Warning message */}
      <Typography
        className="uds-desc mb-8 mt-0 italic"
        variant="body2"
        color="warning.main"
      >
        {t('settings.modals.dns.dialog.warning')}
      </Typography>

      {visualization ? (
        <List>
          <DnsGeneralFields values={values} onChange={handleChange} />
          <DnsNameserverFields values={values} onChange={handleChange} />
          <DnsFallbackFields values={values} onChange={handleChange} />
          <DnsHostsFields values={values} onChange={handleChange} />
        </List>
      ) : (
        <MonacoEditor
          height="100vh"
          language="yaml"
          value={yamlContent}
          theme={themeMode === 'light' ? 'light' : 'vs-dark'}
          className="flex-grow"
          onMount={(editorInstance) => {
            editorRef.current = editorInstance
          }}
          options={{
            tabSize: 2,
            minimap: {
              enabled: document.documentElement.clientWidth >= 1500,
            },
            mouseWheelZoom: true,
            quickSuggestions: {
              strings: true,
              comments: true,
              other: true,
            },
            padding: {
              top: 33,
            },
            fontFamily: `Fira Code, JetBrains Mono, Roboto Mono, "Source Code Pro", Consolas, Menlo, Monaco, monospace, "Courier New", "Apple Color Emoji"${
              getSystem() === 'windows' ? ', twemoji mozilla' : ''
            }`,
            fontLigatures: false,
            smoothScrolling: true,
          }}
          onChange={handleYamlChange}
        />
      )}
    </BaseDialog>
  )
}
