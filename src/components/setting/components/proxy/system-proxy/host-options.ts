import { getNetworkInterfacesInfo, getSystemHostname } from '@/services/cmds'
import { debugLog } from '@/utils/misc'

import { FALLBACK_HOST_OPTIONS } from './constants'

const extractIpAddresses = (interfaces: INetworkInterface[]) => {
  const ipAddresses: string[] = []

  interfaces.forEach((iface) => {
    iface.addr.forEach((address) => {
      if (address.V4?.ip) {
        ipAddresses.push(address.V4.ip)
      }
      if (address.V6?.ip) {
        ipAddresses.push(address.V6.ip)
      }
    })
  })

  return ipAddresses
}

export async function loadSystemProxyHostOptions() {
  try {
    const interfaces = await getNetworkInterfacesInfo()
    const ipAddresses = extractIpAddresses(interfaces)

    let hostname = ''
    try {
      hostname = await getSystemHostname()
      debugLog('获取到主机名:', hostname)
    } catch (error) {
      console.error('获取主机名失败:', error)
    }

    const options = [...FALLBACK_HOST_OPTIONS]

    if (hostname && !options.includes(hostname)) {
      const localHostname = `${hostname}.local`
      options.push(localHostname)
      debugLog('主机名已加入选项:', localHostname)
    } else if (!hostname) {
      debugLog('主机名为空')
    }

    options.push(...ipAddresses)

    const uniqueOptions = Array.from(new Set(options))
    debugLog('最终代理主机选项:', uniqueOptions)
    return uniqueOptions
  } catch (error) {
    console.error('获取网络接口失败:', error)
    return FALLBACK_HOST_OPTIONS
  }
}
