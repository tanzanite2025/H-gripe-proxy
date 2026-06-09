import './assets/styles/tailwind.css'
import './assets/styles/index.scss'

import { ResizeObserver } from '@juggle/resize-observer'
import { QueryClientProvider } from '@tanstack/react-query'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { ComposeContextProvider } from 'foxact/compose-context-provider'
import React from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from 'react-router'
import { MihomoWebSocket } from 'tauri-plugin-mihomo-api'

import { BaseErrorBoundary } from './components/base'
import { FALLBACK_LANGUAGE, initializeLanguage } from './services/i18n'
import { preloadLanguage } from './services/preload'
import { queryClient } from './services/query-client'
import { disableWebViewShortcuts } from './utils/misc/disable-webview-shortcuts'

if (!window.ResizeObserver) {
  window.ResizeObserver = ResizeObserver
}

const mainElementId = 'root'
const container = document.getElementById(mainElementId)

if (!container) {
  throw new Error(`No container '${mainElementId}' found to render application`)
}

disableWebViewShortcuts()

const isWebSandboxPath =
  typeof window !== 'undefined' &&
  /^\/web-test(?:\/|$)/.test(window.location.pathname)

const getSafeCurrentWebviewWindow = () => {
  try {
    return getCurrentWebviewWindow()
  } catch {
    return null
  }
}

const renderMainApp = async () => {
  const [
    { router },
    { AppDataProvider },
    { WindowProvider },
    { LoadingCacheProvider, UpdateStateProvider },
  ] = await Promise.all([
    import('./pages/_core/router'),
    import('./providers/app-data-provider'),
    import('./providers/window'),
    import('./services/states'),
  ])

  const contexts = [
    <LoadingCacheProvider key="loading" />,
    <UpdateStateProvider key="update" />,
  ]

  const root = createRoot(container)
  root.render(
    <React.StrictMode>
      <ComposeContextProvider contexts={contexts}>
        <BaseErrorBoundary>
          <QueryClientProvider client={queryClient}>
            <WindowProvider>
              <AppDataProvider>
                <RouterProvider router={router} />
              </AppDataProvider>
            </WindowProvider>
          </QueryClientProvider>
        </BaseErrorBoundary>
      </ComposeContextProvider>
    </React.StrictMode>,
  )
}

const renderWebSandbox = async () => {
  const { default: WebTestPage } = await import('./pages/web-test')

  const root = createRoot(container)
  root.render(
    <React.StrictMode>
      <BaseErrorBoundary>
        <QueryClientProvider client={queryClient}>
          <WebTestPage />
        </QueryClientProvider>
      </BaseErrorBoundary>
    </React.StrictMode>,
  )
}

const initializeApp = async () => {
  if (isWebSandboxPath) {
    await renderWebSandbox()
    return
  }

  await renderMainApp()
}

const bootstrap = async () => {
  const initialLanguage = isWebSandboxPath
    ? await preloadLanguage(null)
    : await preloadLanguage()

  await initializeLanguage(initialLanguage)

  if (!isWebSandboxPath) {
    getSafeCurrentWebviewWindow()?.setTheme('dark').catch(() => {})
  }

  await initializeApp()
}

bootstrap().catch((error) => {
  console.error('[main.tsx] App bootstrap failed:', error)
  initializeLanguage(FALLBACK_LANGUAGE)
    .catch((fallbackError) => {
      console.error(
        '[main.tsx] Fallback language initialization failed:',
        fallbackError,
      )
    })
    .finally(() => {
      void initializeApp()
    })
})

window.addEventListener('error', (event) => {
  console.error('[main.tsx] Global error:', event.error)
})

window.addEventListener('unhandledrejection', (event) => {
  console.error('[main.tsx] Unhandled promise rejection:', event.reason)
})

window.addEventListener('beforeunload', () => {
  MihomoWebSocket.cleanupAll()
})
