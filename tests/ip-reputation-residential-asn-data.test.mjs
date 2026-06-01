import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const repoRoot = process.cwd()
const asnDataDir = join(repoRoot, 'src-tauri', 'src', 'core', 'asn_data')
const residentialPath = join(asnDataDir, 'residential.rs')
const modPath = join(asnDataDir, 'mod.rs')
const classifierPath = join(repoRoot, 'src-tauri', 'src', 'core', 'asn_classifier.rs')

test('asn classifier has positive residential ISP data instead of relying on fallback', () => {
  assert.ok(existsSync(residentialPath), 'residential ASN data module should exist')

  const residential = readFileSync(residentialPath, 'utf8')
  const mod = readFileSync(modPath, 'utf8')
  const classifier = readFileSync(classifierPath, 'utf8')

  assert.match(mod, /pub mod residential;/)
  assert.match(residential, /pub fn residential_asns\(\)/)
  assert.match(residential, /pub fn residential_keywords\(\)/)
  assert.match(residential, /\(7922,\s*"Comcast/)
  assert.match(residential, /\(7018,\s*"AT&T/)
  assert.match(residential, /"comcast"/)
  assert.match(residential, /"xfinity"/)
  assert.match(residential, /"charter"/)
  assert.match(residential, /"spectrum"/)

  assert.match(classifier, /asn_data::residential::residential_asns\(\)/)
  assert.match(classifier, /asn_data::residential::residential_keywords\(\)/)
  assert.match(
    classifier,
    /classify_by_org_name\("Comcast Cable"\), AsnCategory::Residential/,
  )
  assert.match(
    classifier,
    /classify_by_org_name\("Charter Communications"\), AsnCategory::Residential/,
  )
  assert.doesNotMatch(
    classifier,
    /classify_by_org_name\("Comcast Cable"\), AsnCategory::Unknown/,
  )
})
