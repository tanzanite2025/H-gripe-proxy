import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const securityComponents = join(repoRoot, 'src', 'components', 'security')
const modelPath = join(securityComponents, 'security-honeypot-decoys.ts')
const controllerPath = join(securityComponents, 'use-security-monitor-controller.ts')

test('security monitor keeps honeypot decoy state behind a model boundary', () => {
  assert.ok(existsSync(modelPath), 'honeypot decoy model should exist')

  const model = readFileSync(modelPath, 'utf8')
  const controller = readFileSync(controllerPath, 'utf8')

  assert.match(model, /export interface HoneypotDecoy/)
  assert.match(model, /export const DEFAULT_HONEYPOT_DECOY_ID/)
  assert.match(model, /export function createDefaultHoneypotDecoys/)
  assert.match(model, /export function getActiveHoneypotDecoyPath/)
  assert.match(model, /export function updateActiveHoneypotDecoyPath/)
  assert.match(model, /export function selectActiveHoneypotDecoyId/)
  assert.match(model, /export function addHoneypotDecoy/)
  assert.match(model, /export function removeHoneypotDecoy/)
  assert.match(model, /export function setHoneypotDecoyEnabled/)

  assert.match(controller, /createDefaultHoneypotDecoys/)
  assert.match(controller, /honeypotDecoys/)
  assert.match(controller, /activeDecoyId/)
  assert.match(controller, /handleAddHoneypotDecoy/)
  assert.match(controller, /handleRemoveHoneypotDecoy/)
  assert.match(controller, /handleHoneypotDecoyEnabledChange/)
  assert.match(controller, /handleActiveDecoyChange/)
  assert.match(controller, /addHoneypotDecoy/)
  assert.match(controller, /removeHoneypotDecoy/)
  assert.match(controller, /setHoneypotDecoyEnabled/)
  assert.match(controller, /getActiveHoneypotDecoyPath/)
  assert.match(controller, /updateActiveHoneypotDecoyPath/)
  assert.match(controller, /selectActiveHoneypotDecoyId/)
  assert.match(controller, /onAddHoneypotDecoy: handleAddHoneypotDecoy/)
  assert.match(controller, /onRemoveHoneypotDecoy: handleRemoveHoneypotDecoy/)
  assert.match(
    controller,
    /onHoneypotDecoyEnabledChange: handleHoneypotDecoyEnabledChange/,
  )
  assert.match(controller, /onActiveDecoyChange: handleActiveDecoyChange/)
  assert.doesNotMatch(controller, /useState\('config_decoy\.yaml'\)/)
  assert.doesNotMatch(controller, /onDecoyPathChange: setDecoyPath/)
})
