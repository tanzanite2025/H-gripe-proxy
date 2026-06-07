import { useLockFn } from 'ahooks'
import yaml from 'js-yaml'
import {
  startTransition,
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react'
import { useTranslation } from 'react-i18next'

import { MonacoEditor } from '@/components/base'
import { Button } from '@/components/tailwind/Button'
import { Dialog } from '@/components/tailwind/Dialog'
import { DialogActions } from '@/components/tailwind/DialogActions'
import { DialogContent } from '@/components/tailwind/DialogContent'
import { DialogTitle } from '@/components/tailwind/DialogTitle'
import { readProfileFile, saveProfileFile } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import type { MonacoEditorInstance } from '@/types/monaco'

import { RuleFormPanel } from './components/rule-form-panel'
import { RuleSequenceList } from './components/rule-sequence-list'
import { builtinProxyPolicies, rules, type RuleDefinition } from './constants'

interface Props {
  groupsUid: string
  mergeUid: string
  profileUid: string
  property: string
  open: boolean
  onClose: () => void
  onSave?: (prev?: string, curr?: string) => void
}

const toArray = <T,>(value: unknown): T[] => {
  return Array.isArray(value) ? (value as T[]) : []
}

const isDeletedGroup = (
  group: IProxyGroupConfig,
  deletedGroups: Array<string | { name: string }>,
) => {
  return deletedGroups.some((item) =>
    typeof item === 'string' ? item === group.name : item.name === group.name,
  )
}

export const RulesEditorViewer = ({
  groupsUid,
  mergeUid,
  profileUid,
  property,
  open,
  onClose,
  onSave,
}: Props) => {
  const { t } = useTranslation()
  const editorRef = useRef<MonacoEditorInstance | null>(null)

  const [prevData, setPrevData] = useState('')
  const [currData, setCurrData] = useState('')
  const [visualization, setVisualization] = useState(true)

  const [ruleType, setRuleType] = useState<RuleDefinition>(rules[0])
  const [ruleContent, setRuleContent] = useState('')
  const [noResolve, setNoResolve] = useState(false)
  const [proxyPolicy, setProxyPolicy] = useState(builtinProxyPolicies[0])
  const [proxyPolicyList, setProxyPolicyList] = useState<string[]>([])
  const [ruleList, setRuleList] = useState<string[]>([])
  const [ruleSetList, setRuleSetList] = useState<string[]>([])
  const [subRuleList, setSubRuleList] = useState<string[]>([])

  const [prependSeq, setPrependSeq] = useState<string[]>([])
  const [appendSeq, setAppendSeq] = useState<string[]>([])
  const [deleteSeq, setDeleteSeq] = useState<string[]>([])

  const fetchContent = useCallback(async () => {
    const data = await readProfileFile(property)
    const config = yaml.load(data) as ISeqProfileConfig | null

    setPrependSeq(config?.prepend || [])
    setAppendSeq(config?.append || [])
    setDeleteSeq(config?.delete || [])
    setPrevData(data)
    setCurrData(data)
  }, [property])

  useEffect(() => {
    if (currData === '' || !visualization) {
      return
    }

    const config = yaml.load(currData) as ISeqProfileConfig | null
    startTransition(() => {
      setPrependSeq(config?.prepend ?? [])
      setAppendSeq(config?.append ?? [])
      setDeleteSeq(config?.delete ?? [])
    })
  }, [currData, visualization])

  useEffect(() => {
    const serialize = () => {
      try {
        setCurrData(
          yaml.dump(
            { prepend: prependSeq, append: appendSeq, delete: deleteSeq },
            { forceQuotes: true },
          ),
        )
      } catch (error) {
        showNotice.error(error ?? 'YAML dump error')
      }
    }

    let idleId: number | undefined
    let timeoutId: number | undefined

    if (window.requestIdleCallback) {
      idleId = window.requestIdleCallback(serialize)
    } else {
      timeoutId = window.setTimeout(serialize, 0)
    }

    return () => {
      if (idleId !== undefined && window.cancelIdleCallback) {
        window.cancelIdleCallback(idleId)
      }
      if (timeoutId !== undefined) {
        clearTimeout(timeoutId)
      }
    }
  }, [prependSeq, appendSeq, deleteSeq])

  const fetchProfile = useCallback(async () => {
    const profileData = await readProfileFile(profileUid)
    const groupsData = await readProfileFile(groupsUid)
    const mergeData = await readProfileFile(mergeUid)
    const globalMergeData = await readProfileFile('Merge')

    const rulesConfig = yaml.load(profileData) as { rules: string[] } | null
    const profileConfig = yaml.load(profileData) as {
      'proxy-groups': IProxyGroupConfig[]
      'rule-providers': Record<string, unknown>
      'sub-rules': Record<string, unknown>
    } | null
    const groupsConfig = yaml.load(groupsData) as ISeqProfileConfig | null
    const mergeConfig = yaml.load(mergeData) as {
      'rule-providers': Record<string, unknown>
      'sub-rules': Record<string, unknown>
    } | null
    const globalMergeConfig = yaml.load(globalMergeData) as {
      'rule-providers': Record<string, unknown>
      'sub-rules': Record<string, unknown>
    } | null

    const originGroups = profileConfig?.['proxy-groups'] ?? []
    const prependGroups = toArray<IProxyGroupConfig>(groupsConfig?.prepend)
    const appendGroups = toArray<IProxyGroupConfig>(groupsConfig?.append)
    const deletedGroups = toArray<string | { name: string }>(groupsConfig?.delete)
    const mergedGroups = prependGroups.concat(
      originGroups.filter((group) => !isDeletedGroup(group, deletedGroups)),
      appendGroups,
    )

    const mergedRuleProviders = Object.assign(
      {},
      profileConfig?.['rule-providers'] ?? {},
      mergeConfig?.['rule-providers'] ?? {},
      globalMergeConfig?.['rule-providers'] ?? {},
    )
    const mergedSubRules = Object.assign(
      {},
      profileConfig?.['sub-rules'] ?? {},
      mergeConfig?.['sub-rules'] ?? {},
      globalMergeConfig?.['sub-rules'] ?? {},
    )

    setProxyPolicyList(
      builtinProxyPolicies.concat(mergedGroups.map((group) => group.name)),
    )
    setRuleSetList(Object.keys(mergedRuleProviders))
    setSubRuleList(Object.keys(mergedSubRules))
    setRuleList(rulesConfig?.rules || [])
  }, [groupsUid, mergeUid, profileUid])

  useEffect(() => {
    if (!open) return
    void fetchContent()
    void fetchProfile()
  }, [fetchContent, fetchProfile, open])

  useEffect(() => {
    return () => {
      editorRef.current?.dispose()
      editorRef.current = null
    }
  }, [])

  const validateRule = () => {
    if ((ruleType.required ?? true) && !ruleContent) {
      throw new Error(
        t('rules.modals.editor.form.validation.conditionRequired'),
      )
    }

    if (ruleType.validator && !ruleType.validator(ruleContent)) {
      throw new Error(t('rules.modals.editor.form.validation.invalidRule'))
    }

    const condition = ruleType.required ?? true ? ruleContent : ''
    return `${ruleType.name}${condition ? ',' + condition : ''},${proxyPolicy}${
      ruleType.noResolve && noResolve ? ',no-resolve' : ''
    }`
  }

  const addRuleToPrepend = () => {
    try {
      const raw = validateRule()
      if (prependSeq.includes(raw)) return
      setPrependSeq([raw, ...prependSeq])
    } catch (error: any) {
      showNotice.error(error)
    }
  }

  const addRuleToAppend = () => {
    try {
      const raw = validateRule()
      if (appendSeq.includes(raw)) return
      setAppendSeq([...appendSeq, raw])
    } catch (error: any) {
      showNotice.error(error)
    }
  }

  const handleSave = useLockFn(async () => {
    try {
      if (!(await saveProfileFile(property, currData))) {
        await fetchContent()
        onClose()
        return
      }
      showNotice.success('shared.feedback.notifications.saved')
      onSave?.(prevData, currData)
      onClose()
    } catch (error: any) {
      showNotice.error(error)
    }
  })

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth="xl"
      fullWidth
      disableEnforceFocus={!visualization}
    >
      <DialogTitle>
        <div className="flex justify-between">
          <span>{t('rules.modals.editor.title')}</span>
          <Button
            variant="contained"
            size="small"
            onClick={() => {
              setVisualization((current) => !current)
            }}
          >
            {visualization
              ? t('shared.editorModes.advanced')
              : t('shared.editorModes.visualization')}
          </Button>
        </div>
      </DialogTitle>

      <DialogContent className="flex h-[calc(100vh-185px)] w-auto">
        {visualization ? (
          <>
            <RuleFormPanel
              ruleType={ruleType}
              ruleContent={ruleContent}
              noResolve={noResolve}
              proxyPolicy={proxyPolicy}
              ruleSetList={ruleSetList}
              subRuleList={subRuleList}
              proxyPolicyList={proxyPolicyList}
              onRuleTypeChange={setRuleType}
              onRuleContentChange={setRuleContent}
              onNoResolveChange={setNoResolve}
              onProxyPolicyChange={setProxyPolicy}
              onAddPrepend={addRuleToPrepend}
              onAddAppend={addRuleToAppend}
            />

            <RuleSequenceList
              prependSeq={prependSeq}
              ruleList={ruleList}
              appendSeq={appendSeq}
              deleteSeq={deleteSeq}
              setPrependSeq={setPrependSeq}
              setAppendSeq={setAppendSeq}
              setDeleteSeq={setDeleteSeq}
            />
          </>
        ) : (
          <MonacoEditor
            height="100%"
            language="yaml"
            value={currData}
            theme="vs-dark"
            onMount={(editorInstance) => {
              editorRef.current = editorInstance
            }}
            options={{
              tabSize: 2,
              minimap: {
                enabled: document.documentElement.clientWidth >= 1500,
              },
              mouseWheelZoom: true,
              quickSuggestions: {
                strings: true,
                comments: true,
                other: true,
              },
              padding: {
                top: 33,
              },
              fontFamily:
                'Josefin Sans, YouSheBiaoTiHei, twemoji mozilla, Segoe UI Emoji, -apple-system, BlinkMacSystemFont, Segoe UI, Microsoft YaHei UI, Microsoft YaHei, Roboto, Helvetica Neue, Arial, sans-serif',
              fontLigatures: false,
              smoothScrolling: true,
            }}
            onChange={(value) => setCurrData(value ?? '')}
          />
        )}
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose} variant="outlined">
          {t('shared.actions.cancel')}
        </Button>

        <Button onClick={handleSave} variant="contained">
          {t('shared.actions.save')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}
