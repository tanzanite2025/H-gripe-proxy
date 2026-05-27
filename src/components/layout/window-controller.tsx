import { forwardRef, useImperativeHandle } from 'react'

import { useWindowControls } from '@/hooks/ui/use-window'
import getSystem from '@/utils/misc'

const controlButtonStyle = {
  width: 28,
  height: 28,
  border: 'none',
  background: 'transparent',
  color: 'inherit',
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  borderRadius: 8,
  cursor: 'default',
  padding: 0,
  fontSize: 14,
  lineHeight: 1,
}

export const WindowControls = forwardRef(function WindowControls(props, ref) {
  const OS = getSystem()
  const {
    currentWindow,
    maximized,
    minimize,
    close,
    toggleFullscreen,
    toggleMaximize,
  } = useWindowControls()

  useImperativeHandle(
    ref,
    () => ({
      currentWindow,
      maximized,
      minimize,
      close,
      toggleFullscreen,
      toggleMaximize,
    }),
    [
      currentWindow,
      maximized,
      minimize,
      close,
      toggleFullscreen,
      toggleMaximize,
    ],
  )

  // 通过前端对 tauri 窗口进行翻转全屏时会短暂地与系统图标重叠渲染。
  // 这可能是上游缺陷，保险起见跨平台以窗口的最大化翻转为准。

  return (
    <div className="flex items-center gap-1">
      {OS === 'macos' && (
        <>
          <button type="button" style={controlButtonStyle} onClick={close}>
            ×
          </button>
          <button type="button" style={controlButtonStyle} onClick={minimize}>
            −
          </button>
          <button type="button" style={controlButtonStyle} onClick={toggleMaximize}>
            {maximized ? '❐' : '□'}
          </button>
        </>
      )}

      {OS === 'windows' && (
        <>
          <button type="button" style={controlButtonStyle} onClick={minimize}>
            −
          </button>
          <button type="button" style={controlButtonStyle} onClick={toggleMaximize}>
            {maximized ? '❐' : '□'}
          </button>
          <button
            type="button"
            style={{ ...controlButtonStyle, color: '#d32f2f' }}
            onClick={close}
          >
            ×
          </button>
        </>
      )}

      {OS === 'linux' && (
        <>
          <button type="button" style={controlButtonStyle} onClick={minimize}>
            −
          </button>
          <button type="button" style={controlButtonStyle} onClick={toggleMaximize}>
            {maximized ? '❐' : '□'}
          </button>
          <button
            type="button"
            style={{ ...controlButtonStyle, color: '#d32f2f' }}
            onClick={close}
          >
            ×
          </button>
        </>
      )}
    </div>
  )
})
