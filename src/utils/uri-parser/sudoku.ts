import {
  decodeAndTrim,
  decodeBase64OrOriginal,
  parseBoolOrPresence,
  parseRequiredPort,
  stripUriScheme,
} from './helpers'

type SudokuShortLinkPayload = {
  h?: string
  p?: number
  k?: string
  a?: string
  e?: string
  m?: number
  x?: boolean
  t?: string
  ts?: string[]
  hd?: boolean
  hm?: string
  ht?: boolean
  hh?: string
  hx?: string
  hy?: string
}

function parseSudokuJson(line: string): SudokuShortLinkPayload {
  const afterScheme = stripUriScheme(line, 'sudoku', 'Invalid sudoku uri')
  if (!afterScheme) {
    throw new Error('Invalid sudoku uri')
  }

  const [encoded, fragmentRaw] = afterScheme.split('#', 2)
  const decoded = decodeBase64OrOriginal(encoded)
  const payload = JSON.parse(decoded) as SudokuShortLinkPayload
  if (fragmentRaw && payload.h !== undefined) {
    ;(payload as SudokuShortLinkPayload & { _name?: string })._name = decodeAndTrim(
      fragmentRaw,
    )
  }
  return payload
}

function normalizeSudokuTableType(value: string | undefined): SudokuTableType {
  const normalized = value?.trim().toLowerCase()
  switch (normalized) {
    case 'ascii':
    case 'prefer_ascii':
      return 'prefer_ascii'
    case 'up_ascii_down_entropy':
      return 'up_ascii_down_entropy'
    case 'up_entropy_down_ascii':
      return 'up_entropy_down_ascii'
    case 'entropy':
    case 'prefer_entropy':
    default:
      return 'prefer_entropy'
  }
}

function normalizeSudokuHttpMaskMode(
  value: string | undefined,
): SudokuHttpMaskMode | undefined {
  const normalized = value?.trim().toLowerCase()
  switch (normalized) {
    case 'legacy':
    case 'stream':
    case 'poll':
    case 'auto':
    case 'ws':
      return normalized
    default:
      return undefined
  }
}

function normalizeSudokuHttpMaskMultiplex(
  value: string | undefined,
): SudokuHttpMaskMultiplex | undefined {
  const normalized = value?.trim().toLowerCase()
  switch (normalized) {
    case 'off':
    case 'auto':
    case 'on':
      return normalized
    default:
      return undefined
  }
}

function normalizeSudokuAeadMethod(
  value: string | undefined,
): SudokuAeadMethod | undefined {
  switch (value?.trim().toLowerCase()) {
    case 'chacha20-poly1305':
    case 'aes-128-gcm':
    case 'none':
      return value.trim().toLowerCase() as SudokuAeadMethod
    default:
      return undefined
  }
}

export function URI_Sudoku(line: string): IProxySudokuConfig {
  const payload = parseSudokuJson(line) as SudokuShortLinkPayload & {
    _name?: string
  }

  if (!payload.h || payload.p === undefined || !payload.k) {
    throw new Error('Invalid sudoku uri')
  }

  const port = parseRequiredPort(payload.p, 'Invalid sudoku uri')
  const name = payload._name ?? `Sudoku ${payload.h}:${port}`
  const proxy: IProxySudokuConfig = {
    type: 'sudoku',
    name,
    server: payload.h,
    port,
    key: payload.k,
    'table-type': normalizeSudokuTableType(payload.a),
    'aead-method': normalizeSudokuAeadMethod(payload.e) ?? 'none',
    'padding-min': 5,
    'padding-max': 15,
    'enable-pure-downlink': !payload.x,
  }

  if (payload.t) {
    proxy['custom-table'] = payload.t
  }
  if (Array.isArray(payload.ts) && payload.ts.length > 0) {
    proxy['custom-tables'] = payload.ts
  }

  const httpmask: NonNullable<IProxySudokuConfig['httpmask']> = {}
  if (payload.hd !== undefined) {
    httpmask.disable = parseBoolOrPresence(String(payload.hd))
  }
  const httpmaskMode = normalizeSudokuHttpMaskMode(payload.hm)
  if (httpmaskMode) {
    httpmask.mode = httpmaskMode
  }
  if (payload.ht !== undefined) {
    httpmask.tls = parseBoolOrPresence(String(payload.ht))
  }
  if (payload.hh) {
    httpmask.host = payload.hh
  }
  if (payload.hy) {
    httpmask['path-root'] = payload.hy
  }
  const httpmaskMultiplex = normalizeSudokuHttpMaskMultiplex(payload.hx)
  if (httpmaskMultiplex) {
    httpmask.multiplex = httpmaskMultiplex
  }
  if (Object.keys(httpmask).length > 0) {
    proxy.httpmask = httpmask
  }

  return proxy
}
