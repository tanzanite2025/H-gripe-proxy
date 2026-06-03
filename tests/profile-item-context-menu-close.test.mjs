import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'

const profileItem = readFileSync(
  new URL('../src/components/profile/profile-item.tsx', import.meta.url),
  'utf8',
)

test('profile item context menu closes before running selected action', () => {
  assert.match(profileItem, /handleContextMenuItemClick/)
  assert.doesNotMatch(profileItem, /onClick=\{item\.handler\}/)
  assert.match(profileItem, /setAnchorEl\(null\)[\s\S]*item\.handler\(\)/)
})
