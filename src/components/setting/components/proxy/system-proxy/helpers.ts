export const getDefaultBypass = (
  systemName: string,
  isWindows: boolean,
) => {
  if (isWindows) {
    return 'localhost;127.*;192.168.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;<local>'
  }
  if (systemName === 'linux') {
    return 'localhost,127.0.0.1,192.168.0.0/16,10.0.0.0/8,172.16.0.0/12,::1'
  }
  return '127.0.0.1,192.168.0.0/16,10.0.0.0/8,172.16.0.0/12,localhost,*.local,*.crashlytics.com,<local>'
}

export const splitBypass = (value?: string) =>
  (value ?? '')
    .split(/[,\n;\r]+/)
    .map((item) => item.trim())
    .filter(Boolean)
