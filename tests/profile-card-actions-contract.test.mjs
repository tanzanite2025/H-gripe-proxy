import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'

const profileItem = readFileSync(
  new URL('../src/components/profile/profile-item.tsx', import.meta.url),
  'utf8',
)
const profileItemUi = readFileSync(
  new URL('../src/components/profile/profile-item-ui.tsx', import.meta.url),
  'utf8',
)
const profileCardActions = readFileSync(
  new URL('../src/components/profile/profile-card-actions.tsx', import.meta.url),
  'utf8',
)

test('profile card exposes translated text actions instead of a single refresh icon', () => {
  assert.match(profileItemUi, /ProfileCardActions/)
  assert.match(profileItemUi, /onUseClick/)
  assert.match(profileItemUi, /onDirectUpdateClick/)
  assert.match(profileItemUi, /onProxyUpdateClick/)
  assert.match(profileItemUi, /onEditProxiesClick/)
  assert.match(profileItemUi, /onEditGroupsClick/)

  for (const key of [
    'profiles.components.menu.select',
    'profiles.components.menu.update',
    'profiles.components.menu.updateViaProxy',
    'profiles.components.menu.editProxies',
    'profiles.components.menu.editGroups',
  ]) {
    assert.match(profileCardActions, new RegExp(key.replaceAll('.', '\\.')))
  }

  assert.match(profileCardActions, /rounded-full/)
})

test('profile item wires card text actions to the same handlers as the menu', () => {
  assert.match(profileItem, /onForceSelect/)
  assert.match(profileItem, /onEditProxies/)
  assert.match(profileItem, /onEditGroups/)
  assert.match(profileItem, /onUpdate\(0\)/)
  assert.match(profileItem, /onUpdate\(2\)/)
  assert.match(profileItem, /canEditProxies=\{!!option\?\.proxies\}/)
  assert.match(profileItem, /canEditGroups=\{!!option\?\.groups\}/)
})

test('context menus omit actions already exposed on the profile card', () => {
  const urlMenu = profileItem.match(
    /const urlModeMenu: ContextMenuItem\[] = \[([\s\S]*?)\]\s*const fileModeMenu/,
  )?.[1]
  const fileMenu = profileItem.match(
    /const fileModeMenu: ContextMenuItem\[] = \[([\s\S]*?)\]\s*\/\/ 监听自动更新事件/,
  )?.[1]

  assert.ok(urlMenu, 'urlModeMenu block should be present')
  assert.ok(fileMenu, 'fileModeMenu block should be present')

  for (const duplicate of [
    'menuLabels.select',
    'menuLabels.editProxies',
    'menuLabels.editGroups',
  ]) {
    assert.doesNotMatch(urlMenu, new RegExp(duplicate.replace('.', '\\.')))
    assert.doesNotMatch(fileMenu, new RegExp(duplicate.replace('.', '\\.')))
  }

  for (const duplicate of ['menuLabels.update', 'menuLabels.updateViaProxy']) {
    assert.doesNotMatch(urlMenu, new RegExp(duplicate.replace('.', '\\.')))
  }
})
