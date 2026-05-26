/**
 * DNS 配置管理 Hook
 * 负责加载、保存、验证和应用 DNS 配置
 */

import { invoke } from '@tauri-apps/api/core'
import yaml from 'js-yaml'
import { useCallback } from 'react'

import { useClash } from '@/hooks/data'
import { showNotice } from '@/services/notice-service'

interface ValidationOutcome {
  status: 'valid' | 'invalid' | 'skipped'
  message?: string
}

export const useDnsConfig = () => {
  const { clash, mutateClash } = useClash()

  /**
   * 加载 DNS 配置
   * @returns 配置对象，如果不存在则返回 null
   */
  const loadConfig = useCallback(async () => {
    try {
      const dnsConfigExists = await invoke<boolean>(
        'check_dns_config_exists',
        {},
      )

      if (dnsConfigExists) {
        const dnsConfig = await invoke<string>('get_dns_config_content', {})
        const config = yaml.load(dnsConfig) as any
        return config
      }

      return null
    } catch (err) {
      console.error('Failed to load DNS config', err)
      return null
    }
  }, [])

  /**
   * 保存 DNS 配置
   * @param config 配置对象
   */
  const saveConfig = useCallback(
    async (config: Record<string, any>) => {
      try {
        // 保存配置
        await invoke('save_dns_config', { dnsConfig: config })

        // 验证配置
        const validation = await invoke<ValidationOutcome>(
          'validate_dns_config',
          {},
        )

        if (validation.status !== 'valid') {
          const errorMsg =
            validation.status === 'invalid'
              ? validation.message
              : 'Configuration validation skipped'
          let cleanErrorMsg = errorMsg || ''

          // 提取关键错误信息
          if (cleanErrorMsg.includes('level=error')) {
            const errorLines = cleanErrorMsg
              .split('\n')
              .filter(
                (line) =>
                  line.includes('level=error') ||
                  line.includes('level=fatal') ||
                  line.includes('failed'),
              )

            if (errorLines.length > 0) {
              cleanErrorMsg = errorLines
                .map((line) => {
                  const msgMatch = line.match(/msg="([^"]+)"/)
                  return msgMatch ? msgMatch[1] : line
                })
                .join(', ')
            }
          }

          showNotice.error(
            'settings.modals.dns.messages.configError',
            cleanErrorMsg,
          )
          return false
        }

        // 如果DNS开关当前是打开的，则需要应用新的DNS配置
        if (clash?.dns?.enable) {
          await invoke('apply_dns_config', { apply: true })
          mutateClash()
        }

        return true
      } catch (err) {
        showNotice.error(err)
        return false
      }
    },
    [clash?.dns?.enable, mutateClash],
  )

  return {
    loadConfig,
    saveConfig,
  }
}
