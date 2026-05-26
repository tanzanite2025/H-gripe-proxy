/**
 * DNS 表单管理 Hook
 * 负责表单状态管理、值转换、YAML 同步
 */

import yaml from 'js-yaml'
import { useCallback, useEffect, useReducer, useRef, useState } from 'react'

import { debugLog } from '@/utils/misc'

import {
  configToFormValues,
  formValuesToConfig,
  getDefaultFormValues,
  type DnsFormValues,
} from '../utils/dns-helpers'

export const useDnsForm = () => {
  const [values, setValues] = useState<DnsFormValues>(getDefaultFormValues())
  const [visualization, setVisualization] = useState(true)
  const skipYamlSyncRef = useRef(false)

  // 用于YAML编辑模式
  const [yamlContent, setYamlContent] = useReducer(
    (_: string, next: string) => next,
    '',
  )

  /**
   * 从配置对象更新表单值
   */
  const updateValuesFromConfig = useCallback(
    (config: any) => {
      if (!config) return
      setValues(configToFormValues(config))
    },
    [setValues],
  )

  /**
   * 从表单值生成配置对象
   */
  const generateConfigFromValues = useCallback(() => {
    return formValuesToConfig(values)
  }, [values])

  /**
   * 从表单值更新 YAML
   */
  const updateYamlFromValues = useCallback(() => {
    const config = formValuesToConfig(values)
    setYamlContent(yaml.dump(config, { forceQuotes: true }))
  }, [values, setYamlContent])

  /**
   * 从 YAML 更新表单值
   */
  const updateValuesFromYaml = useCallback(() => {
    try {
      const parsedYaml = yaml.load(yamlContent) as any
      if (!parsedYaml) return

      skipYamlSyncRef.current = true
      updateValuesFromConfig(parsedYaml)
    } catch (err) {
      debugLog('YAML 解析错误', err)
    }
  }, [yamlContent, updateValuesFromConfig])

  /**
   * 重置为默认值
   */
  const resetToDefaults = useCallback(() => {
    setValues(getDefaultFormValues())
    updateYamlFromValues()
  }, [setValues, updateYamlFromValues])

  // 当表单值变化时，自动更新 YAML（跳过从 YAML 触发的更新）
  useEffect(() => {
    if (skipYamlSyncRef.current) {
      skipYamlSyncRef.current = false
      return
    }
    updateYamlFromValues()
  }, [updateYamlFromValues])

  // 保存最新的更新函数引用
  const latestUpdateValuesFromYamlRef = useRef(updateValuesFromYaml)
  const latestUpdateYamlFromValuesRef = useRef(updateYamlFromValues)

  useEffect(() => {
    latestUpdateValuesFromYamlRef.current = updateValuesFromYaml
    latestUpdateYamlFromValuesRef.current = updateYamlFromValues
  }, [updateValuesFromYaml, updateYamlFromValues])

  // 当切换可视化/YAML 模式时，同步数据
  useEffect(() => {
    if (visualization) {
      latestUpdateValuesFromYamlRef.current()
    } else {
      latestUpdateYamlFromValuesRef.current()
    }
  }, [visualization])

  return {
    values,
    setValues,
    yamlContent,
    setYamlContent,
    visualization,
    setVisualization,
    updateValuesFromConfig,
    generateConfigFromValues,
    updateValuesFromYaml,
    updateYamlFromValues,
    resetToDefaults,
  }
}
