import { Minus, Square, X, Copy } from 'lucide-react'
import { forwardRef, useImperativeHandle } from 'react'

import { useWindowControls } from '@/hooks/ui/use-window'
import getSystem from '@/utils/misc'

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

  const btnBase =
    'inline-flex items-center justify-center w-8 h-8 rounded-lg transition-all duration-200 text-gray-400 hover:text-[#00ff41] hover:bg-[rgba(0,255,65,0.1)]'

  const closeBtn =
    'inline-flex items-center justify-center w-8 h-8 rounded-lg transition-all duration-200 text-gray-400 hover:text-white hover:bg-red-600'

  return (
    <div className="flex items-center gap-1">
      {OS === 'macos' && (
        <>
          <button type="button" className={closeBtn} onClick={close}>
            <X className="w-3.5 h-3.5" />
          </button>
          <button type="button" className={btnBase} onClick={minimize}>
            <Minus className="w-3.5 h-3.5" />
          </button>
          <button type="button" className={btnBase} onClick={toggleMaximize}>
            {maximized ? <Copy className="w-3.5 h-3.5" /> : <Square className="w-3.5 h-3.5" />}
          </button>
        </>
      )}

      {OS === 'windows' && (
        <>
          <button type="button" className={btnBase} onClick={minimize}>
            <Minus className="w-3.5 h-3.5" />
          </button>
          <button type="button" className={btnBase} onClick={toggleMaximize}>
            {maximized ? <Copy className="w-3.5 h-3.5" /> : <Square className="w-3.5 h-3.5" />}
          </button>
          <button type="button" className={closeBtn} onClick={close}>
            <X className="w-3.5 h-3.5" />
          </button>
        </>
      )}

      {OS === 'linux' && (
        <>
          <button type="button" className={btnBase} onClick={minimize}>
            <Minus className="w-3.5 h-3.5" />
          </button>
          <button type="button" className={btnBase} onClick={toggleMaximize}>
            {maximized ? <Copy className="w-3.5 h-3.5" /> : <Square className="w-3.5 h-3.5" />}
          </button>
          <button type="button" className={closeBtn} onClick={close}>
            <X className="w-3.5 h-3.5" />
          </button>
        </>
      )}
    </div>
  )
})
