import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const componentPath = join(repoRoot, 'src', 'components', 'home', 'ip-info-card.tsx')

test('ip info card distinguishes reputation states instead of collapsing them to Unknown', () => {
  const component = readFileSync(componentPath, 'utf8')

  assert.match(
    component,
    /error:\s*reputationError/,
    'reputation query error should be read explicitly',
  )
  assert.match(
    component,
    /formatReputationSummary/,
    'summary formatting should be isolated from JSX',
  )
  assert.match(component, /检测失败/)
  assert.match(component, /未获取 IP/)
  assert.match(component, /结果异常/)
  assert.match(component, /检测中/)

  assert.doesNotMatch(
    component,
    /:\s*'Unknown'\s*(?:\r?\n\s*)$/,
    'reputation fallback should not be a generic Unknown',
  )
  assert.match(
    component,
    /riskColorMap\[reputation\.riskLevel\] \?\?/,
    'unknown risk levels should not create an undefined class lookup',
  )
})
