import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'

const profilePage = readFileSync(
  new URL('../src/pages/profiles.tsx', import.meta.url),
  'utf8',
)
const appDataProvider = readFileSync(
  new URL('../src/providers/app-data-provider.tsx', import.meta.url),
  'utf8',
)

test('profile switch explicitly refreshes runtime rules after a successful activation', () => {
  assert.match(profilePage, /useAppRefreshers/)
  assert.match(profilePage, /const \{ refreshRules, refreshRuleProviders \} = useAppRefreshers\(\)/)
  assert.match(profilePage, /await refreshRules\(\)/)
  assert.match(profilePage, /await refreshRuleProviders\(\)/)

  const successBlock = profilePage.match(
    /const success = await requestPromise[\s\S]*?if \(notifySuccess && success\)/,
  )?.[0]
  assert.ok(successBlock, 'profile activation success block should be present')
  assert.match(successBlock, /await refreshRules\(\)/)
  assert.match(successBlock, /await refreshRuleProviders\(\)/)
})

test('global profile-changed event still refreshes runtime rules as a fallback', () => {
  assert.match(appDataProvider, /listen<string>\(\s*'profile-changed'/)
  assert.match(appDataProvider, /const refreshCoreData = useCallback/)
  assert.match(appDataProvider, /refreshRules\(\)\.catch\(\(\) => \{\}\)/)
  assert.match(appDataProvider, /refreshRuleProviders\(\)\.catch\(\(\) => \{\}\)/)
})
