import {
  Box,
  Chip,
  FormControl,
  InputLabel,
  MenuItem,
  Select,
  SelectChangeEvent,
  Typography,
  alpha,
  useTheme,
} from '@mui/material'
import { useTranslation } from 'react-i18next'

import delayManager from '@/services/delay'

import type { ProxyState } from '../hooks/use-current-proxy-data'
import { convertDelayColor } from '../utils/proxy-helpers'

interface ProxySelectorsProps {
  state: ProxyState
  proxyOptions: Array<{ name: string }>
  isGlobalMode: boolean
  isDirectMode: boolean
  onGroupChange: (event: SelectChangeEvent<string>) => void
  onProxyChange: (event: SelectChangeEvent<string>) => void
}

/**
 * 代理选择器组件
 * 包含代理组选择器和代理节点选择器
 */
export const ProxySelectors = ({
  state,
  proxyOptions,
  isGlobalMode,
  isDirectMode,
  onGroupChange,
  onProxyChange,
}: ProxySelectorsProps) => {
  const { t } = useTranslation()
  const theme = useTheme()

  // 自定义渲染选择框中的值
  const renderProxyValue = (selected: string) => {
    if (!selected || !state.proxyData.records[selected]) return selected

    const delayValue = delayManager.getDelayFix(
      state.proxyData.records[selected],
      state.selection.group,
    )

    return (
      <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
        <Typography noWrap>{selected}</Typography>
        <Chip
          size="small"
          label={delayManager.formatDelay(delayValue)}
          color={convertDelayColor(delayValue)}
        />
      </Box>
    )
  }

  const selectStyles = {
    bgcolor: alpha(theme.palette.action.hover, 0.02),
    borderRadius: '16px',
    '& .MuiOutlinedInput-notchedOutline': {
      borderStyle: 'dashed',
      borderColor: 'divider',
    },
    '&:hover .MuiOutlinedInput-notchedOutline': {
      borderStyle: 'dashed',
      borderColor: 'divider',
    },
    '&.Mui-focused .MuiOutlinedInput-notchedOutline': {
      borderStyle: 'dashed',
      borderWidth: '1px',
      borderColor: 'primary.main',
    },
  }

  return (
    <>
      {/* 代理组选择器 */}
      <FormControl fullWidth variant="outlined" size="small" sx={{ mb: 1.5 }}>
        <InputLabel id="proxy-group-select-label" className="uds-label">
          {t('home.components.currentProxy.labels.group')}
        </InputLabel>
        <Select
          labelId="proxy-group-select-label"
          value={state.selection.group}
          onChange={onGroupChange}
          label={t('home.components.currentProxy.labels.group')}
          disabled={isGlobalMode || isDirectMode}
          sx={selectStyles}
        >
          {state.proxyData.groups.map((group) => (
            <MenuItem key={group.name} value={group.name}>
              {group.name}
            </MenuItem>
          ))}
        </Select>
      </FormControl>

      {/* 代理节点选择器 */}
      <FormControl fullWidth variant="outlined" size="small" sx={{ mb: 0 }}>
        <InputLabel id="proxy-select-label" className="uds-label">
          {t('home.components.currentProxy.labels.proxy')}
        </InputLabel>
        <Select
          labelId="proxy-select-label"
          value={state.selection.proxy}
          onChange={onProxyChange}
          label={t('home.components.currentProxy.labels.proxy')}
          disabled={isDirectMode}
          renderValue={renderProxyValue}
          sx={selectStyles}
          MenuProps={{
            slotProps: {
              paper: {
                style: {
                  maxHeight: 500,
                },
              },
            },
          }}
        >
          {isDirectMode
            ? null
            : proxyOptions.map((proxy) => {
                const delayValue =
                  state.proxyData.records[proxy.name] && state.selection.group
                    ? delayManager.getDelayFix(
                        state.proxyData.records[proxy.name],
                        state.selection.group,
                      )
                    : -1
                return (
                  <MenuItem
                    key={proxy.name}
                    value={proxy.name}
                    sx={{
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'center',
                      width: '100%',
                      pr: 1,
                    }}
                  >
                    <Typography noWrap sx={{ flex: 1, mr: 1 }}>
                      {proxy.name}
                    </Typography>
                    <Chip
                      size="small"
                      label={delayManager.formatDelay(delayValue)}
                      color={convertDelayColor(delayValue)}
                      sx={{
                        minWidth: '60px',
                        height: '22px',
                        flexShrink: 0,
                      }}
                    />
                  </MenuItem>
                )
              })}
        </Select>
      </FormControl>
    </>
  )
}
