import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const coordinatorPath = join(repoRoot, 'src-tauri', 'src', 'core', 'coordinator.rs')
const coordinatorFeaturePath = join(repoRoot, 'src-tauri', 'src', 'feat', 'coordinator.rs')
const securityPolicyFeaturePath = join(repoRoot, 'src-tauri', 'src', 'feat', 'security_policy.rs')
const advancedPagePath = join(repoRoot, 'src', 'pages', 'advanced.tsx')
const securityPolicyPanelPath = join(repoRoot, 'src', 'components', 'advanced', 'security-policy-panel.tsx')

test('coordinator syncs persisted security policies into the runtime manager on hydrate and apply', () => {
  const coordinator = readFileSync(coordinatorPath, 'utf8')

  assert.match(coordinator, /fn sync_security_policies_from_advanced_config\(&self, config: &AdvancedConfig\)/)
  assert.match(
    coordinator,
    /pub fn hydrate_from_advanced_config\(&self, config: &AdvancedConfig\) -> Result<\(\)>\s*\{[\s\S]*?sync_security_policies_from_advanced_config\(config\);[\s\S]*?\}/,
  )
  assert.match(
    coordinator,
    /pub fn apply_advanced_config\(&self, config: &AdvancedConfig\) -> Result<\(\)>\s*\{[\s\S]*?sync_security_policies_from_advanced_config\(config\);[\s\S]*?\}/,
  )
})

test('security policy feature surface only exposes manager-backed runtime actions', () => {
  const feature = readFileSync(securityPolicyFeaturePath, 'utf8')

  assert.match(feature, /pub async fn security_policy_apply\(/)
  assert.match(feature, /pub async fn security_policy_revoke\(/)
  assert.match(feature, /pub async fn security_policy_apply_all\(/)
  assert.match(feature, /pub async fn security_policy_revoke_all\(/)
  assert.match(feature, /pub async fn security_policy_get_states\(/)
  assert.match(feature, /pub async fn security_policy_get_state\(/)
  assert.doesNotMatch(feature, /security_policy_get_policies/)
  assert.doesNotMatch(feature, /security_policy_get\(/)
  assert.doesNotMatch(feature, /security_policy_upsert/)
  assert.doesNotMatch(feature, /security_policy_remove/)
  assert.doesNotMatch(feature, /security_policy_reload/)
})

test('advanced config saves revoke removed applied security policies before syncing runtime config', () => {
  const coordinatorFeature = readFileSync(coordinatorFeaturePath, 'utf8')

  assert.match(coordinatorFeature, /async fn revoke_removed_security_policies\(/)
  assert.match(
    coordinatorFeature,
    /sync_coordinator_from_advanced_config\(\)[\s\S]*?revoke_removed_security_policies_blocking\(&config\)\?;[\s\S]*?COORDINATOR\.hydrate_from_advanced_config\(&config\)/,
  )
  assert.match(
    coordinatorFeature,
    /save_advanced_config\(config: &AdvancedConfig\)[\s\S]*?revoke_removed_security_policies\(config\)\.await\?;[\s\S]*?COORDINATOR\.apply_advanced_config\(config\)\?/,
  )
  assert.match(
    coordinatorFeature,
    /sync_coordinator_from_advanced_config_async\(\)[\s\S]*?revoke_removed_security_policies\(&config\)\.await\?;[\s\S]*?COORDINATOR\.hydrate_from_advanced_config\(&config\)\?/,
  )
})

test('security policy config CRUD commands are removed from the runtime command surface', () => {
  const frontend = readFileSync(join(repoRoot, 'src', 'services', 'cmds.ts'), 'utf8')
  const commandModule = readFileSync(join(repoRoot, 'src-tauri', 'src', 'cmd', 'security_policy.rs'), 'utf8')
  const commandRegistry = readFileSync(join(repoRoot, 'src-tauri', 'src', 'lib.rs'), 'utf8')

  assert.doesNotMatch(frontend, /securityPolicyGetPolicies/)
  assert.doesNotMatch(frontend, /securityPolicyGet\(name: string\)/)
  assert.doesNotMatch(frontend, /securityPolicyUpsert/)
  assert.doesNotMatch(frontend, /securityPolicyRemove/)
  assert.doesNotMatch(frontend, /securityPolicyReload/)

  assert.doesNotMatch(commandModule, /security_policy_get_policies/)
  assert.doesNotMatch(commandModule, /security_policy_get\(name: String\)/)
  assert.doesNotMatch(commandModule, /security_policy_upsert/)
  assert.doesNotMatch(commandModule, /security_policy_remove/)
  assert.doesNotMatch(commandModule, /security_policy_reload/)

  assert.doesNotMatch(commandRegistry, /cmd::security_policy_get_policies/)
  assert.doesNotMatch(commandRegistry, /cmd::security_policy_get,/)
  assert.doesNotMatch(commandRegistry, /cmd::security_policy_upsert/)
  assert.doesNotMatch(commandRegistry, /cmd::security_policy_remove/)
  assert.doesNotMatch(commandRegistry, /cmd::security_policy_reload/)
})

test('security policy runtime actions are gated when the editor has unsaved policy changes', () => {
  const advancedPage = readFileSync(advancedPagePath, 'utf8')
  const panel = readFileSync(securityPolicyPanelPath, 'utf8')

  assert.match(advancedPage, /const hasUnsavedSecurityPolicies =/)
  assert.match(advancedPage, /<SecurityPolicyPanel[\s\S]*hasUnsavedChanges=\{hasUnsavedSecurityPolicies\}/)
  assert.match(panel, /hasUnsavedChanges\?: boolean/)
  assert.match(panel, /disabled=\{loading \|\| hasUnsavedChanges \|\| !policy\.enabled\}/)
  assert.match(panel, /disabled=\{loading \|\| hasUnsavedChanges\}/)
  assert.match(panel, /save the configuration first/i)
})

test('reapplying a policy revokes existing runtime rules before creating replacements', () => {
  const runtimeCore = readFileSync(join(repoRoot, 'src-tauri', 'src', 'core', 'security_policy.rs'), 'utf8')

  assert.match(
    runtimeCore,
    /pub async fn apply_policy\(policy: &SecurityPolicy\) -> Result<Vec<i32>> \{[\s\S]*?get_applied_state\(&policy\.name\)\.await[\s\S]*?revoke_policy\(&policy\.name\)\.await\?;[\s\S]*?create_rule/s,
  )
})
