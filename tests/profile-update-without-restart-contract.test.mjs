import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const profileFeaturePath = join(repoRoot, 'src-tauri', 'src', 'feat', 'profile.rs')
const coreConfigPath = join(repoRoot, 'src-tauri', 'src', 'core', 'manager', 'config.rs')

test('subscription update uses no-restart config apply to avoid network-breaking core restart', () => {
  const profileFeature = readFileSync(profileFeaturePath, 'utf8')

  assert.match(profileFeature, /update_config_without_restart_with_force\(is_mannual_trigger\)/)
  assert.doesNotMatch(profileFeature, /update_config_with_force\(is_mannual_trigger\)/)
})

test('subscription update snapshot tolerates a missing current profile file', () => {
  const profileFeature = readFileSync(profileFeaturePath, 'utf8')
  const snapshot = profileFeature.match(
    /async fn snapshot_profile_update\(uid: &String\) -> Result<Option<ProfileUpdateSnapshot>> \{[\s\S]*?\n\}/,
  )

  assert.ok(snapshot, 'snapshot_profile_update should exist')
  assert.match(snapshot[0], /try_exists\(&path\)\.await/)
  assert.match(snapshot[0], /Ok\(false\) => \{[\s\S]*?None[\s\S]*?\}/)
})

test('no-restart config apply never restarts core after reload failure', () => {
  const coreConfig = readFileSync(coreConfigPath, 'utf8')
  const method = coreConfig.match(
    /pub async fn apply_generate_config_without_restart\(&self\)[\s\S]*?(?=\n    async fn apply_config|\n    pub async fn|\n})/,
  )

  assert.ok(method, 'apply_generate_config_without_restart should exist')
  assert.match(method[0], /validate_config_outcome\(\)/)
  assert.match(method[0], /RunningMode::NotRunning/)
  assert.match(method[0], /self\.start_core\(\)\.await/)
  assert.match(method[0], /self\.reload_config\(path\)\.await/)
  assert.match(method[0], /Config::generate_file\(ConfigType::Run\)/)
  assert.match(method[0], /Config::runtime\(\)\.await\.apply\(\)/)
  assert.match(method[0], /Config::runtime\(\)\.await\.discard\(\)/)
  assert.doesNotMatch(method[0], /restart_core\(\)/)
  assert.doesNotMatch(method[0], /apply_config\(/)
})
