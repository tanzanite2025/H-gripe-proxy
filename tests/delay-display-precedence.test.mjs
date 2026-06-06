import assert from 'node:assert/strict'
import { test } from 'node:test'

import {
  getLatestHistoryDelayUpdate,
  resolveDisplayDelayUpdate,
} from '../src/services/delay-display.ts'

test('uses normal runtime history when cached delay is Error', () => {
  const cached = { delay: 1e6, updatedAt: 1000 }
  const proxy = {
    history: [{ delay: 128, time: '2026-06-06T03:00:00.000Z' }],
  }

  assert.equal(resolveDisplayDelayUpdate(cached, proxy)?.delay, 128)
})

test('keeps active testing state over runtime history', () => {
  const cached = { delay: -2, updatedAt: 1000 }
  const proxy = {
    history: [{ delay: 128, time: '2026-06-06T03:00:00.000Z' }],
  }

  assert.equal(resolveDisplayDelayUpdate(cached, proxy)?.delay, -2)
})

test('keeps successful cached measurements over older runtime history', () => {
  const cached = { delay: 88, updatedAt: 1000 }
  const proxy = {
    history: [{ delay: 128, time: '2026-06-06T03:00:00.000Z' }],
  }

  assert.equal(resolveDisplayDelayUpdate(cached, proxy)?.delay, 88)
})

test('converts latest runtime history to a display update', () => {
  const proxy = {
    history: [
      { delay: 320, time: '2026-06-06T03:00:00.000Z' },
      { delay: 180, time: '2026-06-06T03:01:00.000Z' },
    ],
  }

  const update = getLatestHistoryDelayUpdate(proxy)

  assert.equal(update?.delay, 180)
  assert.ok(update?.updatedAt)
})
