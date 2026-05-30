import type { ChangeEvent, ReactNode } from 'react'
import { Control, Controller, UseFormReturn } from 'react-hook-form'
import { useTranslation } from 'react-i18next'

import { Switch } from '@/components/base'
import { Button } from '@/components/tailwind/Button'
import { InputAdornment } from '@/components/tailwind/InputAdornment'
import { List } from '@/components/tailwind/List'
import { ListItem } from '@/components/tailwind/ListItem'
import { ListItemText } from '@/components/tailwind/ListItemText'
import { Select } from '@/components/tailwind/Select'
import type { SelectChangeEvent } from '@/components/tailwind/Select'
import { TextField } from '@/components/tailwind/TextField'

interface FieldRowProps {
  label: ReactNode
  children: ReactNode
}

const FieldRow = ({ label, children }: FieldRowProps) => (
  <ListItem className="py-1.5 px-0.5">
    <div className="w-[200px] shrink-0 pr-2">
      <ListItemText primary={label} />
    </div>
    <div className="flex-1 min-w-0">{children}</div>
  </ListItem>
)

interface GroupFormProps {
  control: Control<IProxyGroupConfig>
  formIns: Omit<UseFormReturn<IProxyGroupConfig>, 'control'>
  proxyPolicyList: string[]
  proxyProviderList: string[]
  interfaceNameList: string[]
  translateStrategy: (value: string) => string
  translatePolicy: (value: string) => string
  onPrepend: () => void
  onAppend: () => void
}

export const GroupForm = ({
  control,
  proxyPolicyList,
  proxyProviderList,
  interfaceNameList,
  translateStrategy,
  translatePolicy,
  onPrepend,
  onAppend,
}: GroupFormProps) => {
  const { t } = useTranslation()

  return (
    <List className="w-1/2 px-2.5">
      <div className="h-[calc(100%-80px)] overflow-y-auto">
        <Controller
          name="type"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t('profiles.modals.groupsEditor.fields.type')}
            >
              <Select
                size="small"
                className="w-full"
                value={field.value}
                onChange={(e) => field.onChange(e.target.value)}
              >
                {['select', 'url-test', 'fallback', 'load-balance', 'relay'].map(
                  (option) => (
                    <option key={option} value={option}>
                      {translateStrategy(option)}
                    </option>
                  ),
                )}
              </Select>
            </FieldRow>
          )}
        />
        <Controller
          name="name"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t('profiles.modals.groupsEditor.fields.name')}
            >
              <TextField
                autoComplete="new-password"
                size="small"
                className="w-full"
                {...field}
                error={field.value === ''}
                required={true}
              />
            </FieldRow>
          )}
        />
        <Controller
          name="icon"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t('profiles.modals.groupsEditor.fields.icon')}
            >
              <TextField
                autoComplete="new-password"
                size="small"
                className="w-full"
                {...field}
              />
            </FieldRow>
          )}
        />
        <Controller
          name="proxies"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t('profiles.modals.groupsEditor.fields.proxies')}
            >
              <Select
                size="small"
                className="w-full"
                multiple
                value={field.value || []}
                onChange={(e) => {
                  const value = e.target.value
                  field.onChange(
                    typeof value === 'string' ? value.split(',') : value,
                  )
                }}
              >
                {proxyPolicyList.map((option) => (
                  <option key={option} value={option}>
                    {translatePolicy(option)}
                  </option>
                ))}
              </Select>
            </FieldRow>
          )}
        />
        <Controller
          name="use"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t('profiles.modals.groupsEditor.fields.provider')}
            >
              <Select
                size="small"
                className="w-full"
                multiple
                value={field.value || []}
                onChange={(e) => {
                  const value = e.target.value
                  field.onChange(
                    typeof value === 'string' ? value.split(',') : value,
                  )
                }}
              >
                {proxyProviderList.map((option) => (
                  <option key={option} value={option}>
                    {option}
                  </option>
                ))}
              </Select>
            </FieldRow>
          )}
        />
        <Controller
          name="url"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t(
                'profiles.modals.groupsEditor.fields.healthCheckUrl',
              )}
            >
              <TextField
                autoComplete="new-password"
                placeholder="http://cp.cloudflare.com/generate_204"
                size="small"
                className="w-full"
                {...field}
              />
            </FieldRow>
          )}
        />
        <Controller
          name="expected-status"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t(
                'profiles.modals.groupsEditor.fields.expectedStatus',
              )}
            >
              <TextField
                autoComplete="new-password"
                placeholder="*"
                size="small"
                className="w-full"
                onChange={(e: ChangeEvent<HTMLInputElement>) => {
                  field.onChange(parseInt(e.target.value))
                }}
              />
            </FieldRow>
          )}
        />
        <Controller
          name="interval"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t('profiles.modals.groupsEditor.fields.interval')}
            >
              <TextField
                autoComplete="new-password"
                placeholder="300"
                type="number"
                size="small"
                className="w-full"
                onChange={(e: ChangeEvent<HTMLInputElement>) => {
                  field.onChange(parseInt(e.target.value))
                }}
                InputProps={{
                  endAdornment: (
                    <InputAdornment position="end">
                      {t('shared.units.seconds')}
                    </InputAdornment>
                  ),
                }}
              />
            </FieldRow>
          )}
        />
        <Controller
          name="timeout"
          control={control}
          render={({ field }) => (
            <FieldRow label={t('shared.labels.timeout')}>
              <TextField
                autoComplete="new-password"
                placeholder="5000"
                type="number"
                size="small"
                className="w-full"
                onChange={(e: ChangeEvent<HTMLInputElement>) => {
                  field.onChange(parseInt(e.target.value))
                }}
                InputProps={{
                  endAdornment: (
                    <InputAdornment position="end">
                      {t('shared.units.milliseconds')}
                    </InputAdornment>
                  ),
                }}
              />
            </FieldRow>
          )}
        />
        <Controller
          name="max-failed-times"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t(
                'profiles.modals.groupsEditor.fields.maxFailedTimes',
              )}
            >
              <TextField
                autoComplete="new-password"
                placeholder="5"
                type="number"
                size="small"
                className="w-full"
                onChange={(e: ChangeEvent<HTMLInputElement>) => {
                  field.onChange(parseInt(e.target.value))
                }}
              />
            </FieldRow>
          )}
        />
        <Controller
          name="interface-name"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t(
                'profiles.modals.groupsEditor.fields.interfaceName',
              )}
            >
              <Select
                size="small"
                className="w-full"
                value={field.value || ''}
                onChange={(e: SelectChangeEvent) => field.onChange(e.target.value)}
              >
                {interfaceNameList.map((option) => (
                  <option key={option} value={option}>
                    {option}
                  </option>
                ))}
              </Select>
            </FieldRow>
          )}
        />
        <Controller
          name="routing-mark"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t('profiles.modals.groupsEditor.fields.routingMark')}
            >
              <TextField
                autoComplete="new-password"
                type="number"
                size="small"
                className="w-full"
                onChange={(e: ChangeEvent<HTMLInputElement>) => {
                  field.onChange(parseInt(e.target.value))
                }}
              />
            </FieldRow>
          )}
        />
        <Controller
          name="filter"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t('profiles.modals.groupsEditor.fields.filter')}
            >
              <TextField
                autoComplete="new-password"
                size="small"
                className="w-full"
                {...field}
              />
            </FieldRow>
          )}
        />
        <Controller
          name="exclude-filter"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t(
                'profiles.modals.groupsEditor.fields.excludeFilter',
              )}
            >
              <TextField
                autoComplete="new-password"
                size="small"
                className="w-full"
                {...field}
              />
            </FieldRow>
          )}
        />
        <Controller
          name="exclude-type"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t('profiles.modals.groupsEditor.fields.excludeType')}
            >
              <Select
                multiple
                size="small"
                className="w-full"
                value={field.value?.split('|') || []}
                onChange={(e) => {
                  const value = e.target.value
                  const arr =
                    typeof value === 'string' ? value.split(',') : value
                  field.onChange(arr.join('|'))
                }}
              >
                {[
                  'Direct',
                  'Reject',
                  'RejectDrop',
                  'Compatible',
                  'Pass',
                  'Dns',
                  'Shadowsocks',
                  'ShadowsocksR',
                  'Snell',
                  'Socks5',
                  'Http',
                  'Vmess',
                  'Vless',
                  'Trojan',
                  'Hysteria',
                  'Hysteria2',
                  'WireGuard',
                  'Tuic',
                  'Mieru',
                  'Masque',
                  'AnyTLS',
                  'Sudoku',
                  'Relay',
                  'Selector',
                  'Fallback',
                  'URLTest',
                  'LoadBalance',
                  'Ssh',
                ].map((option) => (
                  <option key={option} value={option}>
                    {option}
                  </option>
                ))}
              </Select>
            </FieldRow>
          )}
        />
        <Controller
          name="include-all"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t('profiles.modals.groupsEditor.fields.includeAll')}
            >
              <Switch checked={field.value} {...field} />
            </FieldRow>
          )}
        />
        <Controller
          name="include-all-proxies"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t(
                'profiles.modals.groupsEditor.fields.includeAllProxies',
              )}
            >
              <Switch checked={field.value} {...field} />
            </FieldRow>
          )}
        />
        <Controller
          name="include-all-providers"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t(
                'profiles.modals.groupsEditor.fields.includeAllProviders',
              )}
            >
              <Switch checked={field.value} {...field} />
            </FieldRow>
          )}
        />
        <Controller
          name="lazy"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t('profiles.modals.groupsEditor.toggles.lazy')}
            >
              <Switch checked={field.value} {...field} />
            </FieldRow>
          )}
        />
        <Controller
          name="disable-udp"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t('profiles.modals.groupsEditor.toggles.disableUdp')}
            >
              <Switch checked={field.value} {...field} />
            </FieldRow>
          )}
        />
        <Controller
          name="hidden"
          control={control}
          render={({ field }) => (
            <FieldRow
              label={t('profiles.modals.groupsEditor.toggles.hidden')}
            >
              <Switch checked={field.value} {...field} />
            </FieldRow>
          )}
        />
      </div>
      <ListItem className="py-1.5 px-0.5">
        <Button fullWidth variant="contained" onClick={onPrepend}>
          <svg
            className="w-5 h-5 mr-2"
            viewBox="0 0 24 24"
            fill="currentColor"
          >
            <path d="M8 11h3v10h2V11h3l-4-4-4 4zM4 3v2h16V3H4z" />
          </svg>
          {t('profiles.modals.groupsEditor.actions.prepend')}
        </Button>
      </ListItem>
      <ListItem className="py-1.5 px-0.5">
        <Button fullWidth variant="contained" onClick={onAppend}>
          <svg
            className="w-5 h-5 mr-2"
            viewBox="0 0 24 24"
            fill="currentColor"
          >
            <path d="M16 13h-3V3h-2v10H8l4 4 4-4zM4 19v2h16v-2H4z" />
          </svg>
          {t('profiles.modals.groupsEditor.actions.append')}
        </Button>
      </ListItem>
    </List>
  )
}
