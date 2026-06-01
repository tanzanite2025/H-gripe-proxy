import assert from 'node:assert/strict'
import { beforeEach, test } from 'node:test'

import { getCachedIpInfo } from '../src/services/ip-cache.ts'

const store = new Map()

Object.defineProperty(globalThis, 'localStorage', {
  value: {
    getItem: (key) => store.get(key) ?? null,
    removeItem: (key) => {
      store.delete(key)
    },
    setItem: (key, value) => {
      store.set(key, value)
    },
  },
  configurable: true,
})

beforeEach(() => {
  store.clear()
})

test('drops cached IP info when the payload is missing the IP address', () => {
  localStorage.setItem(
    'clash-verge-ip-info',
    JSON.stringify({
      data: {
        country: 'Wonderland',
        country_code: 'WL',
        lastFetchTs: Date.now(),
      },
      timestamp: Date.now(),
    }),
  )

  assert.equal(getCachedIpInfo(), null)
  assert.equal(localStorage.getItem('clash-verge-ip-info'), null)
})
