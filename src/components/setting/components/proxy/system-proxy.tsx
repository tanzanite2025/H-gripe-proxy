import { useLockFn } from 'ahooks'
import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
} from 'react'

import { DialogRef } from '@/components/base'
import { SystemProxyUI } from './system-proxy-ui'
import { useSystemProxyState, useVerge } from '@/hooks/system'
import { useClashConfigData, useSystemData } from '@/providers/app-data-context'
import {
  getAutotemProxy,
  getNetworkInterfacesInfo,
  getSystemHostname,
  getSystemProxy,
  patchVergeConfig,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { debugLog } from '@/utils/misc'
import getSystem from '@/utils/misc'

const sleep = (ms: number) =>
  new Promise<void>((resolve) => {
    setTimeout(resolve, ms)
  })

const DEFAULT_PAC = `function FindProxyForURL(url, host) {
  return "PROXY %proxy_host%:%mixed-port%; SOCKS5 %proxy_host%:%mixed-port%; DIRECT;";
}`

/** NO_PROXY validation */

// *., cdn*., *, etc.
const domain_subdomain_part = String.raw`(?:[a-z0-9\-\*]+\.|\*)*`
// .*, .cn, .moe, .co*, *
const domain_tld_part = String.raw`(?:\w{2,64}\*?|\*)`
// *epicgames*, *skk.moe, *.skk.moe, skk.*, sponsor.cdn.skk.moe, *.*, etc.
// also matches 192.168.*, 10.*, 127.0.0.*, etc. (partial ipv4)
const rDomainSimple = domain_subdomain_part + domain_tld_part

const ipv4_part = String.raw`\d{1,3}`

const ipv6_part = '(?:[a-fA-F0-9:])+'

const rLocal = `localhost|<local>|localdomain`

const getValidReg = (isWindows: boolean) => {
  // 127.0.0.1 (full ipv4)
  const rIPv4Unix = String.raw`(?:${ipv4_part}\.){3}${ipv4_part}(?:\/\d{1,2})?`
  const rIPv4Windows = String.raw`(?:${ipv4_part}\.){3}${ipv4_part}`

  const rIPv6Unix = String.raw`(?:${ipv6_part}:+)+${ipv6_part}(?:\/\d{1,3})?`
  const rIPv6Windows = String.raw`(?:${ipv6_part}:+)+${ipv6_part}`

  const rValidPart = `${rDomainSimple}|${
    isWindows ? rIPv4Windows : rIPv4Unix
  }|${isWindows ? rIPv6Windows : rIPv6Unix}|${rLocal}`
  const separator = isWindows ? ';' : ','
  const rValid = String.raw`^(${rValidPart})(?:${separator}\s?(${rValidPart}))*${separator}?$`

  return new RegExp(rValid)
}

const splitBypass = (value?: string) =>
  (value ?? '')
    .split(/[,\n;\r]+/)
    .map((item) => item.trim())
    .filter(Boolean)

export const SysproxyViewer = forwardRef<DialogRef>((props, ref) => {
  const systemName = getSystem()
  const isWindows = systemName === 'windows'
  const validReg = useMemo(() => getValidReg(isWindows), [isWindows])

  const [open, setOpen] = useState(false)
  const [editorOpen, setEditorOpen] = useState(false)
  const [pacEditorValue, setPacEditorValue] = useState(DEFAULT_PAC)
  const [pacEditorSavedValue, setPacEditorSavedValue] = useState(DEFAULT_PAC)
  const [saving, setSaving] = useState(false)
  const { verge, patchVerge, mutateVerge } = useVerge()
  const [hostOptions, setHostOptions] = useState<string[]>([])

  const { clashConfig } = useClashConfigData()
  const { indicator: isProxyReallyEnabled, invalidateProxyState } =
    useSystemProxyState()

  const {
    enable_system_proxy: enabled,
    proxy_auto_config,
    pac_file_content,
    enable_proxy_guard,
    enable_bypass_check,
    use_default_bypass,
    system_proxy_bypass,
    proxy_guard_duration,
    proxy_host,
  } = verge ?? {}

  const [value, setValue] = useState({
    guard: enable_proxy_guard ?? false,
    enable_bypass_check: enable_bypass_check ?? true,
    bypass: system_proxy_bypass ?? '',
    duration: proxy_guard_duration ?? 10,
    use_default: use_default_bypass ?? true,
    pac: proxy_auto_config ?? false,
    pac_content: pac_file_content ?? DEFAULT_PAC,
    proxy_host: proxy_host ?? '127.0.0.1',
  })

  const separator = useMemo(() => (isWindows ? ';' : ','), [isWindows])

  const defaultBypass = () => {
    if (isWindows) {
      return 'localhost;127.*;192.168.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;<local>'
    }
    if (systemName === 'linux') {
      return 'localhost,127.0.0.1,192.168.0.0/16,10.0.0.0/8,172.16.0.0/12,::1'
    }
    return '127.0.0.1,192.168.0.0/16,10.0.0.0/8,172.16.0.0/12,localhost,*.local,*.crashlytics.com,<local>'
  }

  const prevMixedPortRef = useRef(clashConfig?.mixedPort)

  useEffect(() => {
    const mixedPort = clashConfig?.mixedPort
    if (!mixedPort || mixedPort === prevMixedPortRef.current) {
      return
    }

    prevMixedPortRef.current = mixedPort
    if (!enabled) {
      return
    }

    const updateProxy = async () => {
      try {
        const currentSysProxy = await getSystemProxy()
        const currentAutoProxy = await getAutotemProxy()

        if (value.pac ? currentAutoProxy?.enable : currentSysProxy?.enable) {
          await patchVergeConfig({ enable_system_proxy: false })
          await sleep(200)
          await patchVergeConfig({ enable_system_proxy: true })
          await invalidateProxyState()
        }
      } catch (err) {
        showNotice.error(err)
      }
    }

    updateProxy()
  }, [clashConfig?.mixedPort, enabled, value.pac, invalidateProxyState])

  const { systemProxyAddress } = useSystemData()

  // 为当前状态计算系统代理地址
  const getSystemProxyAddress = useMemo(() => {
    if (!clashConfig) return '-'

    const isPacMode = value.pac ?? false

    if (isPacMode) {
      const host = value.proxy_host || '127.0.0.1'
      const port = verge?.verge_mixed_port || clashConfig.mixedPort || 7897
      return `${host}:${port}`
    } else {
      return systemProxyAddress
    }
  }, [
    value.pac,
    value.proxy_host,
    verge?.verge_mixed_port,
    clashConfig,
    systemProxyAddress,
  ])
  const getCurrentPacUrl = useMemo(() => {
    const host = value.proxy_host || '127.0.0.1'
    // 根据环境判断PAC端口
    const port = import.meta.env.DEV ? 11233 : 33331
    return `http://${host}:${port}/commands/pac`
  }, [value.proxy_host])

  const bypassError =
    value.enable_bypass_check &&
    !value.pac &&
    !value.use_default &&
    value.bypass
      ? !validReg.test(value.bypass)
      : false

  const openPacEditor = () => {
    const nextPac = value.pac_content ?? DEFAULT_PAC
    setPacEditorValue(nextPac)
    setPacEditorSavedValue(nextPac)
    setEditorOpen(true)
  }

  const handleSavePac = useLockFn(async () => {
    const nextPac =
      pacEditorValue.trim().length > 0 ? pacEditorValue : DEFAULT_PAC

    setValue((current) => ({ ...current, pac_content: nextPac }))
    setPacEditorSavedValue(nextPac)
  })

  useImperativeHandle(ref, () => ({
    open: () => {
      setOpen(true)
      setValue({
        guard: enable_proxy_guard ?? false,
        enable_bypass_check: enable_bypass_check ?? true,
        bypass: system_proxy_bypass ?? '',
        duration: proxy_guard_duration ?? 10,
        use_default: use_default_bypass ?? true,
        pac: proxy_auto_config ?? false,
        pac_content: pac_file_content ?? DEFAULT_PAC,
        proxy_host: proxy_host ?? '127.0.0.1',
      })
      fetchNetworkInterfaces()
    },
    close: () => setOpen(false),
  }))

  // 获取网络接口和主机名
  const fetchNetworkInterfaces = async () => {
    try {
      // 获取系统网络接口信息
      const interfaces = await getNetworkInterfacesInfo()
      const ipAddresses: string[] = []

      // 从interfaces中提取IPv4和IPv6地址
      interfaces.forEach((iface) => {
        iface.addr.forEach((address) => {
          if (address.V4 && address.V4.ip) {
            ipAddresses.push(address.V4.ip)
          }
          if (address.V6 && address.V6.ip) {
            ipAddresses.push(address.V6.ip)
          }
        })
      })

      // 获取当前系统的主机名
      let hostname = ''
      try {
        hostname = await getSystemHostname()
        debugLog('获取到主机名:', hostname)
      } catch (err) {
        console.error('获取主机名失败:', err)
      }

      // 构建选项列表
      const options = ['127.0.0.1', 'localhost']

      // 确保主机名添加到列表，即使它是空字符串也记录下来
      if (hostname) {
        // 如果主机名不是localhost或127.0.0.1，则添加它
        if (hostname !== 'localhost' && hostname !== '127.0.0.1') {
          hostname = hostname + '.local'
          options.push(hostname)
          debugLog('主机名已添加到选项中:', hostname)
        } else {
          debugLog('主机名与已有选项重复:', hostname)
        }
      } else {
        debugLog('主机名为空')
      }

      // 添加IP地址
      options.push(...ipAddresses)

      // 去重
      const uniqueOptions = Array.from(new Set(options))
      debugLog('最终选项列表:', uniqueOptions)
      setHostOptions(uniqueOptions)
    } catch (error) {
      console.error('获取网络接口失败:', error)
      // 失败时至少提供基本选项
      setHostOptions(['127.0.0.1', 'localhost'])
    }
  }

  const onSave = useLockFn(async () => {
    if (value.duration < 1) {
      showNotice.error('settings.modals.sysproxy.messages.durationTooShort')
      return
    }
    if (
      value.enable_bypass_check &&
      !value.pac &&
      !value.use_default &&
      value.bypass &&
      !validReg.test(value.bypass)
    ) {
      showNotice.error('settings.modals.sysproxy.messages.invalidBypass')
      return
    }

    // 修改验证规则，允许IP和主机名
    const ipv4Regex =
      /^((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/
    const ipv6Regex =
      /^(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))$/
    const hostnameRegex =
      /^(([a-zA-Z0-9]|[a-zA-Z0-9][a-zA-Z0-9-]*[a-zA-Z0-9])\.)*([A-Za-z0-9]|[A-Za-z0-9][A-Za-z0-9-]*[A-Za-z0-9])$/

    if (
      !ipv4Regex.test(value.proxy_host) &&
      !ipv6Regex.test(value.proxy_host) &&
      !hostnameRegex.test(value.proxy_host)
    ) {
      showNotice.error('settings.modals.sysproxy.messages.invalidProxyHost')
      return
    }

    setSaving(true)
    setOpen(false)
    setSaving(false)
    const patch: Partial<IVergeConfig> = {}

    if (value.guard !== enable_proxy_guard) {
      patch.enable_proxy_guard = value.guard
    }
    if (value.enable_bypass_check !== enable_bypass_check) {
      patch.enable_bypass_check = value.enable_bypass_check
    }
    if (value.duration !== proxy_guard_duration) {
      patch.proxy_guard_duration = value.duration
    }
    if (value.bypass !== system_proxy_bypass) {
      patch.system_proxy_bypass = value.bypass
    }
    if (value.pac !== proxy_auto_config) {
      patch.proxy_auto_config = value.pac
    }
    if (value.use_default !== use_default_bypass) {
      patch.use_default_bypass = value.use_default
    }

    let pacContent = value.pac_content
    if (pacContent) {
      pacContent = pacContent.replace(/%proxy_host%/g, value.proxy_host)
      // 将 mixed-port 转换为字符串
      const mixedPortStr = (clashConfig?.mixedPort || '').toString()
      pacContent = pacContent.replace(/%mixed-port%/g, mixedPortStr)
    }

    if (pacContent !== pac_file_content) {
      patch.pac_file_content = pacContent
    }

    // 处理IPv6地址，如果是IPv6地址但没有被方括号包围，则添加方括号
    let proxyHost = value.proxy_host
    if (
      ipv6Regex.test(proxyHost) &&
      !proxyHost.startsWith('[') &&
      !proxyHost.endsWith(']')
    ) {
      proxyHost = `[${proxyHost}]`
    }

    if (proxyHost !== proxy_host) {
      patch.proxy_host = proxyHost
    }

    // 判断是否需要重置系统代理
    const needResetProxy =
      value.pac !== proxy_auto_config ||
      proxyHost !== proxy_host ||
      pacContent !== pac_file_content ||
      value.bypass !== system_proxy_bypass ||
      value.use_default !== use_default_bypass

    Promise.resolve().then(async () => {
      try {
        // 乐观更新本地状态
        if (Object.keys(patch).length > 0) {
          mutateVerge({ ...verge, ...patch }, false)
        }
        if (Object.keys(patch).length > 0) {
          await patchVerge(patch)
        }
        setTimeout(async () => {
          try {
            await invalidateProxyState()

            // 如果需要重置代理且代理当前启用
            if (needResetProxy && enabled) {
              const [currentSysProxy, currentAutoProxy] = await Promise.all([
                getSystemProxy(),
                getAutotemProxy(),
              ])

              const isProxyActive = value.pac
                ? currentAutoProxy?.enable
                : currentSysProxy?.enable

              if (isProxyActive) {
                await patchVergeConfig({ enable_system_proxy: false })
                await new Promise((resolve) => setTimeout(resolve, 50))
                await patchVergeConfig({ enable_system_proxy: true })
                await invalidateProxyState()
              }
            }
          } catch (err) {
            console.warn('代理状态更新失败:', err)
          }
        }, 50)
      } catch (err) {
        console.error('配置保存失败:', err)
        mutateVerge()
        showNotice.error(err)
        // setOpen(true);
      }
    })
  })

  return (
    <SystemProxyUI
      open={open}
      saving={saving}
      enabled={enabled ?? false}
      value={value}
      isProxyReallyEnabled={isProxyReallyEnabled}
      getSystemProxyAddress={getSystemProxyAddress}
      getCurrentPacUrl={getCurrentPacUrl}
      bypassError={bypassError}
      separator={separator}
      hostOptions={hostOptions}
      editorOpen={editorOpen}
      pacEditorValue={pacEditorValue}
      pacEditorSavedValue={pacEditorSavedValue}
      defaultBypass={defaultBypass}
      onClose={() => setOpen(false)}
      onSave={onSave}
      setValue={setValue}
      openPacEditor={openPacEditor}
      setEditorOpen={setEditorOpen}
      setPacEditorValue={setPacEditorValue}
      handleSavePac={handleSavePac}
    />
  )
})
