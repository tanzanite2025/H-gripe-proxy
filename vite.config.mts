import path from 'node:path'

import legacy from '@vitejs/plugin-legacy'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'
import svgr from 'vite-plugin-svgr'

export default defineConfig({
  root: 'src',
  server: { host: '127.0.0.1', port: 3500 },
  plugins: [
    svgr(),
    react({
      jsxImportSource: '@emotion/react',
      babel: {
        plugins: [
          [
            '@emotion/babel-plugin',
            {
              // 确保在 production 构建时也注入样式
              sourceMap: true,
              autoLabel: 'dev-only',
              labelFormat: '[local]',
              // 关键：强制使用 DOM 样式注入而非 speedy 模式
              importMap: {
                '@mui/material': {
                  styled: {
                    canonicalImport: ['@emotion/styled', 'default'],
                  },
                },
              },
            },
          ],
        ],
      },
    }),
    legacy({
      modernTargets: ['edge>=109', 'safari>=14'],
      renderLegacyChunks: false,
      modernPolyfills: ['es.object.has-own', 'web.structured-clone'],
      additionalModernPolyfills: [
        path.resolve('./src/polyfills/matchMedia.js'),
        path.resolve('./src/polyfills/WeakRef.js'),
        path.resolve('./src/polyfills/RegExp.js'),
      ],
    }),
  ],
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    chunkSizeWarningLimit: 4000,
  },
  resolve: {
    alias: {
      '@': path.resolve('./src'),
      '@root': path.resolve('.'),
    },
  },
  define: {
    OS_PLATFORM: `"${process.platform}"`,
  },
})
