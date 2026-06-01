import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const securityComponents = join(repoRoot, 'src', 'components', 'security')
const componentPath = join(securityComponents, 'security-monitor.tsx')
const controllerPath = join(securityComponents, 'use-security-monitor-controller.ts')
const actionsPath = join(securityComponents, 'security-monitor-actions.ts')
const actionDomainDir = join(securityComponents, 'security-actions')

test('security monitor delegates state and actions to a controller hook', () => {
  assert.ok(existsSync(controllerPath), 'controller hook should exist')
  assert.ok(existsSync(actionsPath), 'security action factory should exist')

  const actionDomainFiles = [
    'monitor-actions.ts',
    'decoy-actions.ts',
    'key-actions.ts',
    'self-destruct-actions.ts',
  ]
  for (const file of actionDomainFiles) {
    assert.ok(existsSync(join(actionDomainDir, file)), `${file} should exist`)
  }

  const component = readFileSync(componentPath, 'utf8')
  const controller = readFileSync(controllerPath, 'utf8')
  const actions = readFileSync(actionsPath, 'utf8')
  const monitorActions = readFileSync(join(actionDomainDir, 'monitor-actions.ts'), 'utf8')
  const decoyActions = readFileSync(join(actionDomainDir, 'decoy-actions.ts'), 'utf8')
  const keyActions = readFileSync(join(actionDomainDir, 'key-actions.ts'), 'utf8')
  const selfDestructActions = readFileSync(
    join(actionDomainDir, 'self-destruct-actions.ts'),
    'utf8',
  )

  assert.match(component, /useSecurityMonitorController/)
  assert.match(component, /<SecurityMonitorUI \{\.\.\.controller\} \/>/)
  assert.doesNotMatch(component, /security(Start|Stop|Check|Deploy|Cleanup|Generate|Self)/)
  assert.doesNotMatch(component, /showNotice/)
  assert.doesNotMatch(component, /listen<SecurityStatus>/)
  assert.doesNotMatch(component, /useState|useEffect/)

  assert.match(controller, /export function useSecurityMonitorController/)
  assert.match(controller, /createSecurityMonitorActions/)
  assert.match(controller, /listen<SecurityStatus>/)
  assert.doesNotMatch(controller, /security(Start|Stop)Monitor/)
  assert.doesNotMatch(controller, /security(Deploy|Cleanup)Decoy/)
  assert.doesNotMatch(controller, /securityGenerateEncryptionKey/)
  assert.doesNotMatch(controller, /securitySelfDestruct/)

  assert.match(actions, /export function createSecurityMonitorActions/)
  assert.match(actions, /createMonitorActions/)
  assert.match(actions, /createDecoyActions/)
  assert.match(actions, /createKeyActions/)
  assert.match(actions, /createSelfDestructActions/)
  assert.doesNotMatch(actions, /security(Start|Stop)Monitor/)
  assert.doesNotMatch(actions, /security(Check|Deploy|Cleanup)Decoy/)
  assert.doesNotMatch(actions, /securityGenerateEncryptionKey/)
  assert.doesNotMatch(actions, /securitySelfDestruct/)

  assert.match(monitorActions, /securityStartMonitor/)
  assert.match(monitorActions, /securityStopMonitor/)
  assert.match(decoyActions, /securityDeployDecoy/)
  assert.match(decoyActions, /securityCleanupDecoy/)
  assert.match(decoyActions, /securityCheckDecoyAccess/)
  assert.match(keyActions, /securityGenerateEncryptionKey/)
  assert.match(keyActions, /navigator\.clipboard\.writeText/)
  assert.match(selfDestructActions, /securitySelfDestruct/)
})
