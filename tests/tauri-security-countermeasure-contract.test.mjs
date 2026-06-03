import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'

const panel = readFileSync(
  new URL('../src/components/security/ingress-countermeasure-panel.tsx', import.meta.url),
  'utf8',
)

const advancedPage = readFileSync(
  new URL('../src/pages/advanced.tsx', import.meta.url),
  'utf8',
)

test('advanced security UI exposes ingress countermeasure controls', () => {
  assert.match(panel, /ingress countermeasure/i)
  assert.match(panel, /classifier/i)
  assert.match(panel, /persona/i)
  assert.match(panel, /deception/i)

  assert.match(advancedPage, /IngressCountermeasurePanel/)
  assert.match(advancedPage, /ingress_countermeasure/)
})
