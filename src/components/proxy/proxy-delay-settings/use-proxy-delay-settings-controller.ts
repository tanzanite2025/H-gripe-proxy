import { useLockFn } from 'ahooks'
import { useEffect, useMemo, useState, type ChangeEvent, type FormEvent } from 'react'

import { useVerge } from '@/hooks/system'
import { showNotice } from '@/services/notice-service'

import {
  areDelaySettingsEqual,
  createDelaySettingsState,
  parsePositiveInt,
  type DelaySettingsFormState,
} from './shared'

export function useProxyDelaySettingsController() {
  const { verge, patchVerge } = useVerge()
  const [saving, setSaving] = useState(false)
  const baselineValues = useMemo(() => createDelaySettingsState(verge), [verge])
  const [values, setValues] = useState<DelaySettingsFormState>(baselineValues)

  useEffect(() => {
    setValues(baselineValues)
  }, [baselineValues])

  const isDirty = useMemo(
    () => !areDelaySettingsEqual(values, baselineValues),
    [baselineValues, values],
  )

  const handleReset = () => {
    setValues(baselineValues)
  }

  const handleLatencyTestChange = (event: ChangeEvent<HTMLInputElement>) => {
    setValues((current) => ({
      ...current,
      defaultLatencyTest: event.target.value,
    }))
  }

  const handleLatencyTimeoutChange = (event: ChangeEvent<HTMLInputElement>) => {
    setValues((current) => ({
      ...current,
      defaultLatencyTimeout: parsePositiveInt(
        event.target.value,
        baselineValues.defaultLatencyTimeout,
      ),
    }))
  }

  const handleSave = useLockFn(async () => {
    setSaving(true)

    try {
      await patchVerge({
        default_latency_test: values.defaultLatencyTest.trim(),
        default_latency_timeout: values.defaultLatencyTimeout,
      })
      showNotice.success('shared.feedback.notifications.saved', 1000)
    } catch (error) {
      showNotice.error(error)
    } finally {
      setSaving(false)
    }
  })

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!isDirty || saving) return
    void handleSave()
  }

  return {
    handleLatencyTestChange,
    handleLatencyTimeoutChange,
    handleReset,
    handleSubmit,
    isDirty,
    saving,
    values,
  }
}
