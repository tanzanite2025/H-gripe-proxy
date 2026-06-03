import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'

const profileFeature = readFileSync(
  new URL('../src-tauri/src/feat/profile.rs', import.meta.url),
  'utf8',
)

test('manual direct profile update does not fall back to proxy paths', () => {
  assert.match(profileFeature, /let strict_direct_update =\s*option/)
  assert.match(profileFeature, /with_proxy == Some\(false\)/)
  assert.match(profileFeature, /self_proxy == Some\(false\)/)
  assert.match(profileFeature, /if strict_direct_update \{[\s\S]*?bail!\(/)
})

test('profile update still keeps proxy fallback for unspecified update mode', () => {
  assert.match(profileFeature, /merged_opt\.get_or_insert_with\(PrfOption::default\)\.self_proxy = Some\(true\)/)
  assert.match(profileFeature, /merged_opt\.get_or_insert_with\(PrfOption::default\)\.with_proxy = Some\(true\)/)
})

test('current profile update rolls back downloaded content when runtime refresh fails', () => {
  assert.match(profileFeature, /struct ProfileUpdateSnapshot/)
  assert.match(profileFeature, /snapshot_profile_update/)
  assert.match(profileFeature, /restore_profile_update_snapshot/)
  assert.match(profileFeature, /rollback_snapshot/)
  assert.match(profileFeature, /update_config_without_restart_with_force\(is_mannual_trigger\)/)
  assert.match(profileFeature, /restore_profile_update_snapshot\(&rollback_snapshot\)\.await/)
})
