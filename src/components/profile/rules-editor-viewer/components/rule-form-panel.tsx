import { type ChangeEvent } from 'react'
import { useTranslation } from 'react-i18next'

import { Switch } from '@/components/base'
import { Button } from '@/components/tailwind/Button'
import { List } from '@/components/tailwind/List'
import { ListItem } from '@/components/tailwind/ListItem'
import { ListItemText } from '@/components/tailwind/ListItemText'
import { Select } from '@/components/tailwind/Select'
import type { SelectChangeEvent } from '@/components/tailwind/Select'
import { TextField } from '@/components/tailwind/TextField'

import {
  PROXY_POLICY_LABEL_KEYS,
  RULE_TYPE_LABEL_KEYS,
  rules,
  type RuleDefinition,
} from '../constants'

interface RuleFormPanelProps {
  ruleType: RuleDefinition
  ruleContent: string
  noResolve: boolean
  proxyPolicy: string
  ruleSetList: string[]
  subRuleList: string[]
  proxyPolicyList: string[]
  onRuleTypeChange: (rule: RuleDefinition) => void
  onRuleContentChange: (value: string) => void
  onNoResolveChange: (value: boolean) => void
  onProxyPolicyChange: (value: string) => void
  onAddPrepend: () => void
  onAddAppend: () => void
}

export function RuleFormPanel({
  ruleType,
  ruleContent,
  noResolve,
  proxyPolicy,
  ruleSetList,
  subRuleList,
  proxyPolicyList,
  onRuleTypeChange,
  onRuleContentChange,
  onNoResolveChange,
  onProxyPolicyChange,
  onAddPrepend,
  onAddAppend,
}: RuleFormPanelProps) {
  const { t } = useTranslation()

  return (
    <List className="w-1/2 px-2.5">
      <ListItem className="px-0.5 py-1.5">
        <ListItemText primary={t('rules.modals.editor.form.labels.type')} />
        <Select
          size="small"
          className="min-w-[240px]"
          value={ruleType.name}
          onChange={(event) => {
            const nextRule = rules.find((rule) => rule.name === event.target.value)
            if (nextRule) {
              onRuleTypeChange(nextRule)
            }
          }}
        >
          {rules.map((option) => (
            <option key={option.name} value={option.name}>
              {t(RULE_TYPE_LABEL_KEYS[option.name] ?? option.name)}
            </option>
          ))}
        </Select>
      </ListItem>

      <ListItem
        className="px-0.5 py-1.5"
        style={{ display: !(ruleType.required ?? true) ? 'none' : '' }}
      >
        <ListItemText primary={t('rules.modals.editor.form.labels.content')} />

        {ruleType.name === 'RULE-SET' && (
          <Select
            size="small"
            className="min-w-[240px]"
            value={ruleContent}
            onChange={(event: SelectChangeEvent) =>
              onRuleContentChange(event.target.value as string)
            }
          >
            {ruleSetList.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </Select>
        )}

        {ruleType.name === 'SUB-RULE' && (
          <Select
            size="small"
            className="min-w-[240px]"
            value={ruleContent}
            onChange={(event) => onRuleContentChange(event.target.value)}
          >
            {subRuleList.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </Select>
        )}

        {ruleType.name !== 'RULE-SET' && ruleType.name !== 'SUB-RULE' && (
          <TextField
            autoComplete="new-password"
            size="small"
            className="min-w-[240px]"
            value={ruleContent}
            required={ruleType.required ?? true}
            error={(ruleType.required ?? true) && !ruleContent}
            placeholder={ruleType.example}
            onChange={(event: ChangeEvent<HTMLInputElement>) =>
              onRuleContentChange(event.target.value)
            }
          />
        )}
      </ListItem>

      <ListItem className="px-0.5 py-1.5">
        <ListItemText
          primary={t('rules.modals.editor.form.labels.proxyPolicy')}
        />
        <Select
          size="small"
          className="min-w-[240px]"
          value={proxyPolicy}
          onChange={(event: SelectChangeEvent) =>
            onProxyPolicyChange(event.target.value as string)
          }
        >
          {proxyPolicyList.map((option) => (
            <option key={option} value={option}>
              {t(PROXY_POLICY_LABEL_KEYS[option] ?? option)}
            </option>
          ))}
        </Select>
      </ListItem>

      {ruleType.noResolve && (
        <ListItem className="px-0.5 py-1.5">
          <ListItemText
            primary={t('rules.modals.editor.form.toggles.noResolve')}
          />
          <Switch
            checked={noResolve}
            onChange={() => onNoResolveChange(!noResolve)}
          />
        </ListItem>
      )}

      <ListItem className="px-0.5 py-1.5">
        <Button fullWidth variant="contained" onClick={onAddPrepend}>
          <svg className="mr-2 h-5 w-5" viewBox="0 0 24 24" fill="currentColor">
            <path d="M8 11h3v10h2V11h3l-4-4-4 4zM4 3v2h16V3H4z" />
          </svg>
          {t('rules.modals.editor.form.actions.prependRule')}
        </Button>
      </ListItem>

      <ListItem className="px-0.5 py-1.5">
        <Button fullWidth variant="contained" onClick={onAddAppend}>
          <svg className="mr-2 h-5 w-5" viewBox="0 0 24 24" fill="currentColor">
            <path d="M16 13h-3V3h-2v10H8l4 4 4-4zM4 19v2h16v-2H4z" />
          </svg>
          {t('rules.modals.editor.form.actions.appendRule')}
        </Button>
      </ListItem>
    </List>
  )
}
