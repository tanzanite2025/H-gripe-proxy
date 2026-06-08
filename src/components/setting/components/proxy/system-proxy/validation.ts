import type { SystemProxyFormValue } from './types'

// *., cdn*., *, etc.
const domainSubdomainPart = String.raw`(?:[a-z0-9\-\*]+\.|\*)*`
// .*, .cn, .moe, .co*, *
const domainTldPart = String.raw`(?:\w{2,64}\*?|\*)`
const domainSimplePattern = domainSubdomainPart + domainTldPart

const ipv4PartPattern = String.raw`\d{1,3}`
const ipv6PartPattern = '(?:[a-fA-F0-9:])+'
const localPattern = `localhost|<local>|localdomain`

const ipv4HostRegex =
  /^((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/
const ipv6HostRegex =
  /^(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))$/
const hostnameRegex =
  /^(([a-zA-Z0-9]|[a-zA-Z0-9][a-zA-Z0-9-]*[a-zA-Z0-9])\.)*([A-Za-z0-9]|[A-Za-z0-9][A-Za-z0-9-]*[A-Za-z0-9])$/

export const createBypassValidator = (isWindows: boolean) => {
  const ipv4Pattern = isWindows
    ? String.raw`(?:${ipv4PartPattern}\.){3}${ipv4PartPattern}`
    : String.raw`(?:${ipv4PartPattern}\.){3}${ipv4PartPattern}(?:\/\d{1,2})?`
  const ipv6Pattern = isWindows
    ? String.raw`(?:${ipv6PartPattern}:+)+${ipv6PartPattern}`
    : String.raw`(?:${ipv6PartPattern}:+)+${ipv6PartPattern}(?:\/\d{1,3})?`

  const validPart = `${domainSimplePattern}|${ipv4Pattern}|${ipv6Pattern}|${localPattern}`
  const separator = isWindows ? ';' : ','
  const validPattern = String.raw`^(${validPart})(?:${separator}\s?(${validPart}))*${separator}?$`

  return new RegExp(validPattern)
}

export const hasInvalidBypassValue = (
  value: SystemProxyFormValue,
  validator: RegExp,
) =>
  value.enable_bypass_check &&
  !value.pac &&
  !value.use_default &&
  !!value.bypass &&
  !validator.test(value.bypass)

export const isValidProxyHost = (value: string) =>
  ipv4HostRegex.test(value) ||
  ipv6HostRegex.test(value) ||
  hostnameRegex.test(value)

export const normalizeProxyHost = (value: string) => {
  if (
    ipv6HostRegex.test(value) &&
    !value.startsWith('[') &&
    !value.endsWith(']')
  ) {
    return `[${value}]`
  }

  return value
}
