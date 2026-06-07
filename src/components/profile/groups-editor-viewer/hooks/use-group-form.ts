import { useCallback } from 'react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'

import { showNotice } from '@/services/notice-service'

import {
  PROXY_POLICY_LABEL_KEYS,
  PROXY_STRATEGY_LABEL_KEYS,
} from '../constants'
import { isGroupNameExists, validateGroupName } from '../utils/group-helpers'

interface UseGroupFormProps {
  prependSeq: IProxyGroupConfig[]
  setPrependSeq: (seq: IProxyGroupConfig[]) => void
  appendSeq: IProxyGroupConfig[]
  setAppendSeq: (seq: IProxyGroupConfig[]) => void
  groupList: IProxyGroupConfig[]
}

export const useGroupForm = ({
  prependSeq,
  setPrependSeq,
  appendSeq,
  setAppendSeq,
  groupList,
}: UseGroupFormProps) => {
  const { t } = useTranslation()

  const { control, ...formIns } = useForm<IProxyGroupConfig>({
    defaultValues: {
      type: 'select',
      name: '',
      interval: 300,
      timeout: 5000,
      'max-failed-times': 5,
      lazy: true,
    },
  })

  const translateStrategy = useCallback(
    (value: string) =>
      PROXY_STRATEGY_LABEL_KEYS[value]
        ? t(PROXY_STRATEGY_LABEL_KEYS[value])
        : value,
    [t],
  )

  const translatePolicy = useCallback(
    (value: string) =>
      PROXY_POLICY_LABEL_KEYS[value]
        ? t(PROXY_POLICY_LABEL_KEYS[value])
        : value,
    [t],
  )

  const validateGroup = useCallback(() => {
    const group = formIns.getValues()
    if (!validateGroupName(group.name)) {
      throw new Error(t('profiles.modals.groupsEditor.errors.nameRequired'))
    }
  }, [formIns, t])

  const handlePrepend = useCallback(() => {
    try {
      validateGroup()
      const groupName = formIns.getValues().name
      if (isGroupNameExists(groupName, prependSeq, groupList, appendSeq)) {
        throw new Error(t('profiles.modals.groupsEditor.errors.nameExists'))
      }
      setPrependSeq([formIns.getValues(), ...prependSeq])
    } catch (err) {
      showNotice.error(err)
    }
  }, [
    validateGroup,
    formIns,
    prependSeq,
    groupList,
    appendSeq,
    setPrependSeq,
    t,
  ])

  const handleAppend = useCallback(() => {
    try {
      validateGroup()
      const groupName = formIns.getValues().name
      if (isGroupNameExists(groupName, prependSeq, groupList, appendSeq)) {
        throw new Error(t('profiles.modals.groupsEditor.errors.nameExists'))
      }
      setAppendSeq([...appendSeq, formIns.getValues()])
    } catch (err) {
      showNotice.error(err)
    }
  }, [
    validateGroup,
    formIns,
    prependSeq,
    groupList,
    appendSeq,
    setAppendSeq,
    t,
  ])

  return {
    control,
    translateStrategy,
    translatePolicy,
    handlePrepend,
    handleAppend,
  }
}
