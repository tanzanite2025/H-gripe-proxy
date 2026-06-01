import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const honeypotDir = join(repoRoot, 'src-tauri', 'src', 'security', 'honeypot')

test('backend dynamic honeypot deployment lives behind honeypot strategy domain', () => {
  const strategyPath = join(honeypotDir, 'strategy.rs')
  assert.ok(existsSync(strategyPath), 'backend honeypot strategy module should exist')

  const honeypotMod = readFileSync(join(honeypotDir, 'mod.rs'), 'utf8')
  const strategy = readFileSync(strategyPath, 'utf8')
  const runtime = readFileSync(
    join(repoRoot, 'src-tauri', 'src', 'core', 'security_runtime.rs'),
    'utf8',
  )
  const commands = readFileSync(
    join(repoRoot, 'src-tauri', 'src', 'cmd', 'security.rs'),
    'utf8',
  )
  const service = readFileSync(join(repoRoot, 'src', 'services', 'security.ts'), 'utf8')
  const appLib = readFileSync(join(repoRoot, 'src-tauri', 'src', 'lib.rs'), 'utf8')
  const decoyActions = readFileSync(
    join(repoRoot, 'src', 'components', 'security', 'security-actions', 'decoy-actions.ts'),
    'utf8',
  )

  assert.match(honeypotMod, /pub mod strategy;/)
  assert.match(honeypotMod, /DecoyDeploymentPlan/)
  assert.match(honeypotMod, /DecoyBatchResult/)

  assert.match(strategy, /pub struct DecoyDeploymentPlan/)
  assert.match(strategy, /pub struct DecoyBatchResult/)
  assert.match(strategy, /deploy_decoy_plan/)
  assert.match(strategy, /cleanup_decoy_plan/)
  assert.match(strategy, /check_decoy_plan_access/)
  assert.match(strategy, /ConfigDecoy::new/)

  assert.match(runtime, /deploy_decoy_plan/)
  assert.match(runtime, /cleanup_decoy_plan/)
  assert.match(runtime, /check_decoy_plan_access/)

  assert.match(commands, /security_deploy_decoy_plan/)
  assert.match(commands, /security_cleanup_decoy_plan/)
  assert.match(commands, /security_check_decoy_plan_access/)
  assert.doesNotMatch(commands, /ConfigDecoy::new/)

  assert.match(appLib, /cmd::security_deploy_decoy_plan/)
  assert.match(appLib, /cmd::security_cleanup_decoy_plan/)
  assert.match(appLib, /cmd::security_check_decoy_plan_access/)

  assert.match(service, /export interface DecoyDeploymentPlan/)
  assert.match(service, /export interface DecoyBatchResult/)
  assert.match(service, /securityDeployDecoyPlan/)
  assert.match(service, /securityCleanupDecoyPlan/)
  assert.match(service, /securityCheckDecoyPlanAccess/)

  assert.match(decoyActions, /securityDeployDecoyPlan/)
  assert.match(decoyActions, /securityCleanupDecoyPlan/)
  assert.match(decoyActions, /securityCheckDecoyPlanAccess/)
  assert.doesNotMatch(decoyActions, /Promise\.all/)
})
