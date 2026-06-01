import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

import {
  filterAndOrderConnections,
  getConnectionViewSpec,
} from '../src/components/connection/connection-page-model.ts'
import {
  createCloseConnectionAction,
} from '../src/components/connection/connection-actions.ts'

const repoRoot = process.cwd()

const createConnection = (id, overrides = {}) => ({
  id,
  metadata: {
    network: 'tcp',
    type: 'HTTP',
    host: `host-${id}.example`,
    sourceIP: '127.0.0.1',
    sourcePort: '5000',
    destinationPort: '443',
    destinationIP: '1.1.1.1',
    remoteDestination: '1.1.1.1',
    process: `process-${id}`,
    processPath: `C:/apps/process-${id}.exe`,
    ...overrides.metadata,
  },
  upload: 0,
  download: 0,
  start: '2026-01-01T00:00:00.000Z',
  chains: ['Proxy-A'],
  rule: 'MATCH',
  rulePayload: '',
  curUpload: 0,
  curDownload: 0,
  ...overrides,
})

test('search mode falls back to stable start-time ordering instead of speed sorting', () => {
  const olderButFaster = createConnection('older', {
    start: '2026-01-01T00:00:00.000Z',
    curUpload: 900,
    metadata: { host: 'needle.example' },
  })
  const newerButSlower = createConnection('newer', {
    start: '2026-01-02T00:00:00.000Z',
    curUpload: 10,
    metadata: { host: 'needle.example' },
  })

  const result = filterAndOrderConnections({
    connections: [olderButFaster, newerButSlower],
    match: (input) => input.includes('needle'),
    orderKey: 'uploadSpeed',
    searchText: 'needle',
    viewMode: 'active',
  })

  assert.deepEqual(
    result.map((connection) => connection.id),
    ['newer', 'older'],
  )
})

test('speed sorting still applies when there is no active search text', () => {
  const slower = createConnection('slower', {
    start: '2026-01-02T00:00:00.000Z',
    curUpload: 10,
  })
  const faster = createConnection('faster', {
    start: '2026-01-01T00:00:00.000Z',
    curUpload: 900,
  })

  const result = filterAndOrderConnections({
    connections: [slower, faster],
    match: () => true,
    orderKey: 'uploadSpeed',
    searchText: '',
    viewMode: 'active',
  })

  assert.deepEqual(
    result.map((connection) => connection.id),
    ['faster', 'slower'],
  )
})

test('close connection action is only exposed for active rows and targets the selected connection', async () => {
  const calls = []
  const activeAction = createCloseConnectionAction({
    closed: false,
    connectionId: 'conn-1',
    onCloseConnection: async (connectionId) => {
      calls.push(connectionId)
    },
  })

  assert.ok(activeAction, 'active rows should expose a close action')
  await activeAction.onAction()
  assert.deepEqual(calls, ['conn-1'])

  const closedAction = createCloseConnectionAction({
    closed: true,
    connectionId: 'conn-2',
    onCloseConnection: async () => {},
  })

  assert.equal(closedAction, null)
})

test('closed view always falls back to stable historical ordering and uses a reduced field set', () => {
  const olderButFaster = createConnection('older', {
    start: '2026-01-01T00:00:00.000Z',
    curUpload: 900,
  })
  const newerButSlower = createConnection('newer', {
    start: '2026-01-02T00:00:00.000Z',
    curUpload: 10,
  })

  const result = filterAndOrderConnections({
    connections: [olderButFaster, newerButSlower],
    match: () => true,
    orderKey: 'uploadSpeed',
    searchText: '',
    viewMode: 'closed',
  })

  assert.deepEqual(
    result.map((connection) => connection.id),
    ['newer', 'older'],
  )

  const closedView = getConnectionViewSpec('closed')
  assert.equal(closedView.showTrafficTotals, false)
  assert.equal(closedView.showSortControl, false)
  assert.equal(closedView.showCloseAllAction, false)
  assert.deepEqual(closedView.tableFields, [
    'host',
    'download',
    'upload',
    'chains',
    'rule',
    'process',
  ])
  assert.deepEqual(closedView.listMetaFields, [
    'process',
    'rule',
    'chains',
    'download',
    'upload',
  ])
  assert.deepEqual(closedView.detailFields, [
    'host',
    'download',
    'upload',
    'chains',
    'rule',
    'process',
    'destination',
    'type',
  ])
})

test('connections page consumes search state and table wiring keeps a direct close affordance', () => {
  const page = readFileSync(join(repoRoot, 'src', 'pages', 'connections.tsx'), 'utf8')
  const item = readFileSync(
    join(repoRoot, 'src', 'components', 'connection', 'connection-item.tsx'),
    'utf8',
  )
  const detail = readFileSync(
    join(repoRoot, 'src', 'components', 'connection', 'connection-detail.tsx'),
    'utf8',
  )
  const table = readFileSync(
    join(repoRoot, 'src', 'components', 'connection', 'connection-table.tsx'),
    'utf8',
  )

  assert.match(page, /type SearchState/)
  assert.match(page, /const handleSearch = useCallback\(/)
  assert.match(page, /setSearchText\(state\.text\)/)
  assert.match(page, /searchText/)
  assert.match(page, /getConnectionViewSpec/)
  assert.match(page, /viewSpec\.showTrafficTotals/)
  assert.match(page, /viewSpec\.showSortControl/)
  assert.match(page, /viewSpec\.showCloseAllAction/)
  assert.match(table, /createCloseConnectionAction/)
  assert.match(table, /tableFields/)
  assert.match(item, /listMetaFields/)
  assert.match(detail, /detailFields/)
})
