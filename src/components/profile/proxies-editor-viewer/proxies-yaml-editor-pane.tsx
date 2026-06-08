import { MonacoEditor } from '@/components/base'

import type { ProxiesYamlEditorPaneProps } from './types'

const EDITOR_FONT_FAMILY =
  'Josefin Sans, YouSheBiaoTiHei, twemoji mozilla, Segoe UI Emoji, -apple-system, BlinkMacSystemFont, Segoe UI, Microsoft YaHei UI, Microsoft YaHei, Roboto, Helvetica Neue, Arial, sans-serif'

export function ProxiesYamlEditorPane({
  currData,
  onChange,
  onMount,
}: ProxiesYamlEditorPaneProps) {
  return (
    <MonacoEditor
      height="100%"
      language="yaml"
      value={currData}
      theme="vs-dark"
      onMount={onMount}
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
        fontFamily: EDITOR_FONT_FAMILY,
        fontLigatures: false,
        smoothScrolling: true,
      }}
      onChange={(value) => onChange(value ?? '')}
    />
  )
}
