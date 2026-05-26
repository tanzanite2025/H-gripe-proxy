import {
  VerticalAlignBottomRounded,
  VerticalAlignTopRounded,
} from '@mui/icons-material'
import {
  Autocomplete,
  Box,
  Button,
  InputAdornment,
  List,
  ListItem,
  ListItemText,
  TextField,
  styled,
} from '@mui/material'
import { Control, Controller, UseFormReturn } from 'react-hook-form'
import { useTranslation } from 'react-i18next'

import { Switch } from '@/components/base'

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
    <List
      sx={{
        width: '50%',
        padding: '0 10px',
      }}
    >
      <Box
        sx={{
          height: 'calc(100% - 80px)',
          overflowY: 'auto',
        }}
      >
        <Controller
          name="type"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t('profiles.modals.groupsEditor.fields.type')}
              />
              <Autocomplete
                size="small"
                sx={{ width: 'calc(100% - 150px)' }}
                options={[
                  'select',
                  'url-test',
                  'fallback',
                  'load-balance',
                  'relay',
                ]}
                value={field.value}
                getOptionLabel={translateStrategy}
                renderOption={(props, option) => {
                  const { key, ...optionProps } = props
                  return (
                    <li
                      key={key}
                      {...optionProps}
                      title={translateStrategy(option)}
                    >
                      {translateStrategy(option)}
                    </li>
                  )
                }}
                onChange={(_, value) => value && field.onChange(value)}
                renderInput={(params) => <TextField {...params} />}
              />
            </Item>
          )}
        />
        <Controller
          name="name"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t('profiles.modals.groupsEditor.fields.name')}
              />
              <TextField
                autoComplete="new-password"
                size="small"
                sx={{ width: 'calc(100% - 150px)' }}
                {...field}
                error={field.value === ''}
                required={true}
              />
            </Item>
          )}
        />
        <Controller
          name="icon"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t('profiles.modals.groupsEditor.fields.icon')}
              />
              <TextField
                autoComplete="new-password"
                size="small"
                sx={{ width: 'calc(100% - 150px)' }}
                {...field}
              />
            </Item>
          )}
        />
        <Controller
          name="proxies"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t('profiles.modals.groupsEditor.fields.proxies')}
              />
              <Autocomplete
                size="small"
                sx={{
                  width: 'calc(100% - 150px)',
                }}
                multiple
                options={proxyPolicyList}
                disableCloseOnSelect
                onChange={(_, value) => value && field.onChange(value)}
                renderInput={(params) => <TextField {...params} />}
                renderOption={(props, option) => {
                  const { key, ...optionProps } = props
                  return (
                    <li
                      key={key}
                      {...optionProps}
                      title={translatePolicy(option)}
                    >
                      {translatePolicy(option)}
                    </li>
                  )
                }}
                getOptionLabel={translatePolicy}
              />
            </Item>
          )}
        />
        <Controller
          name="use"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t('profiles.modals.groupsEditor.fields.provider')}
              />
              <Autocomplete
                size="small"
                sx={{ width: 'calc(100% - 150px)' }}
                multiple
                options={proxyProviderList}
                disableCloseOnSelect
                onChange={(_, value) => value && field.onChange(value)}
                renderInput={(params) => <TextField {...params} />}
              />
            </Item>
          )}
        />
        <Controller
          name="url"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t(
                  'profiles.modals.groupsEditor.fields.healthCheckUrl',
                )}
              />
              <TextField
                autoComplete="new-password"
                placeholder="http://cp.cloudflare.com/generate_204"
                size="small"
                sx={{ width: 'calc(100% - 150px)' }}
                {...field}
              />
            </Item>
          )}
        />
        <Controller
          name="expected-status"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t(
                  'profiles.modals.groupsEditor.fields.expectedStatus',
                )}
              />
              <TextField
                autoComplete="new-password"
                placeholder="*"
                size="small"
                sx={{ width: 'calc(100% - 150px)' }}
                onChange={(e) => {
                  field.onChange(parseInt(e.target.value))
                }}
              />
            </Item>
          )}
        />
        <Controller
          name="interval"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t('profiles.modals.groupsEditor.fields.interval')}
              />
              <TextField
                autoComplete="new-password"
                placeholder="300"
                type="number"
                size="small"
                sx={{ width: 'calc(100% - 150px)' }}
                onChange={(e) => {
                  field.onChange(parseInt(e.target.value))
                }}
                slotProps={{
                  input: {
                    endAdornment: (
                      <InputAdornment position="end">
                        {t('shared.units.seconds')}
                      </InputAdornment>
                    ),
                  },
                }}
              />
            </Item>
          )}
        />
        <Controller
          name="timeout"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText primary={t('shared.labels.timeout')} />
              <TextField
                autoComplete="new-password"
                placeholder="5000"
                type="number"
                size="small"
                sx={{ width: 'calc(100% - 150px)' }}
                onChange={(e) => {
                  field.onChange(parseInt(e.target.value))
                }}
                slotProps={{
                  input: {
                    endAdornment: (
                      <InputAdornment position="end">
                        {t('shared.units.milliseconds')}
                      </InputAdornment>
                    ),
                  },
                }}
              />
            </Item>
          )}
        />
        <Controller
          name="max-failed-times"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t(
                  'profiles.modals.groupsEditor.fields.maxFailedTimes',
                )}
              />
              <TextField
                autoComplete="new-password"
                placeholder="5"
                type="number"
                size="small"
                sx={{ width: 'calc(100% - 150px)' }}
                onChange={(e) => {
                  field.onChange(parseInt(e.target.value))
                }}
              />
            </Item>
          )}
        />
        <Controller
          name="interface-name"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t(
                  'profiles.modals.groupsEditor.fields.interfaceName',
                )}
              />
              <Autocomplete
                size="small"
                sx={{ width: 'calc(100% - 150px)' }}
                options={interfaceNameList}
                value={field.value}
                onChange={(_, value) => value && field.onChange(value)}
                renderInput={(params) => <TextField {...params} />}
              />
            </Item>
          )}
        />
        <Controller
          name="routing-mark"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t('profiles.modals.groupsEditor.fields.routingMark')}
              />
              <TextField
                autoComplete="new-password"
                type="number"
                size="small"
                sx={{ width: 'calc(100% - 150px)' }}
                onChange={(e) => {
                  field.onChange(parseInt(e.target.value))
                }}
              />
            </Item>
          )}
        />
        <Controller
          name="filter"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t('profiles.modals.groupsEditor.fields.filter')}
              />
              <TextField
                autoComplete="new-password"
                size="small"
                sx={{ width: 'calc(100% - 150px)' }}
                {...field}
              />
            </Item>
          )}
        />
        <Controller
          name="exclude-filter"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t(
                  'profiles.modals.groupsEditor.fields.excludeFilter',
                )}
              />
              <TextField
                autoComplete="new-password"
                size="small"
                sx={{ width: 'calc(100% - 150px)' }}
                {...field}
              />
            </Item>
          )}
        />
        <Controller
          name="exclude-type"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t('profiles.modals.groupsEditor.fields.excludeType')}
              />
              <Autocomplete
                multiple
                options={[
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
                ]}
                size="small"
                disableCloseOnSelect
                sx={{ width: 'calc(100% - 150px)' }}
                value={field.value?.split('|')}
                onChange={(_, value) => {
                  field.onChange(value.join('|'))
                }}
                renderInput={(params) => <TextField {...params} />}
              />
            </Item>
          )}
        />
        <Controller
          name="include-all"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t('profiles.modals.groupsEditor.fields.includeAll')}
              />
              <Switch checked={field.value} {...field} />
            </Item>
          )}
        />
        <Controller
          name="include-all-proxies"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t(
                  'profiles.modals.groupsEditor.fields.includeAllProxies',
                )}
              />
              <Switch checked={field.value} {...field} />
            </Item>
          )}
        />
        <Controller
          name="include-all-providers"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t(
                  'profiles.modals.groupsEditor.fields.includeAllProviders',
                )}
              />
              <Switch checked={field.value} {...field} />
            </Item>
          )}
        />
        <Controller
          name="lazy"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t('profiles.modals.groupsEditor.toggles.lazy')}
              />
              <Switch checked={field.value} {...field} />
            </Item>
          )}
        />
        <Controller
          name="disable-udp"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t('profiles.modals.groupsEditor.toggles.disableUdp')}
              />
              <Switch checked={field.value} {...field} />
            </Item>
          )}
        />
        <Controller
          name="hidden"
          control={control}
          render={({ field }) => (
            <Item>
              <ListItemText
                primary={t('profiles.modals.groupsEditor.toggles.hidden')}
              />
              <Switch checked={field.value} {...field} />
            </Item>
          )}
        />
      </Box>
      <Item>
        <Button
          fullWidth
          variant="contained"
          startIcon={<VerticalAlignTopRounded />}
          onClick={onPrepend}
        >
          {t('profiles.modals.groupsEditor.actions.prepend')}
        </Button>
      </Item>
      <Item>
        <Button
          fullWidth
          variant="contained"
          startIcon={<VerticalAlignBottomRounded />}
          onClick={onAppend}
        >
          {t('profiles.modals.groupsEditor.actions.append')}
        </Button>
      </Item>
    </List>
  )
}

const Item = styled(ListItem)(() => ({
  padding: '5px 2px',
}))
