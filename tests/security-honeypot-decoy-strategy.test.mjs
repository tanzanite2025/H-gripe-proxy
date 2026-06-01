import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const securityComponents = join(repoRoot, 'src', 'components', 'security')
const strategyPath = join(securityComponents, 'security-honeypot-decoy-strategy.ts')
const modelPath = join(securityComponents, 'security-honeypot-decoys.ts')
const controllerPath = join(securityComponents, 'use-security-monitor-controller.ts')

test('dynamic honeypot decoy generation lives behind a strategy boundary', () => {
  assert.ok(existsSync(strategyPath), 'honeypot decoy strategy should exist')

  const strategy = readFileSync(strategyPath, 'utf8')
  const model = readFileSync(modelPath, 'utf8')
  const controller = readFileSync(controllerPath, 'utf8')

  assert.match(strategy, /export interface HoneypotDecoyStrategyProfile/)
  assert.match(strategy, /export function createHoneypotDecoyStrategyProfile/)
  assert.match(strategy, /export function generateHoneypotDecoysFromStrategy/)
  assert.match(strategy, /export function mergeHoneypotDecoyStrategy/)
  assert.match(strategy, /config_decoy\.yaml/)
  assert.match(strategy, /profiles/)
  assert.match(strategy, /providers/)
  assert.match(strategy, /rules/)

  assert.match(model, /export function normalizeActiveHoneypotDecoyId/)
  assert.match(model, /getEnabledHoneypotDecoys/)

  assert.match(controller, /normalizeActiveHoneypotDecoyId/)
  assert.match(controller, /mergeHoneypotDecoyStrategy/)
  assert.match(controller, /handleApplyHoneypotDecoyStrategy/)
  assert.match(controller, /setActiveDecoyId\(\(currentDecoyId\)/)
  assert.match(controller, /onApplyHoneypotDecoyStrategy: handleApplyHoneypotDecoyStrategy/)
  assert.doesNotMatch(controller, /profiles\/|providers\/|rules\//)
})
