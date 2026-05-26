import {
  Box,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Radio,
  RadioGroup,
  Select,
  Typography,
} from '@mui/material'
import { useState } from 'react'

type MieruMultiplexing =
  | 'MULTIPLEXING_OFF'
  | 'MULTIPLEXING_LOW'
  | 'MULTIPLEXING_MIDDLE'
  | 'MULTIPLEXING_HIGH'

interface MieruMultiplexConfigProps {
  multiplexing: MieruMultiplexing
  onChange: (multiplexing: MieruMultiplexing) => void
}

const multiplexingLevels = [
  {
    value: 'MULTIPLEXING_OFF' as const,
    label: '关闭',
    description: '不使用多路复用',
  },
  {
    value: 'MULTIPLEXING_LOW' as const,
    label: '低',
    description: '最小的多路复用，适合低延迟场景',
  },
  {
    value: 'MULTIPLEXING_MIDDLE' as const,
    label: '中',
    description: '平衡性能和资源，推荐使用',
  },
  {
    value: 'MULTIPLEXING_HIGH' as const,
    label: '高',
    description: '最大的多路复用，适合高吞吐场景',
  },
]

export function MieruMultiplexConfig({
  multiplexing,
  onChange,
}: MieruMultiplexConfigProps) {
  const [localMultiplexing, setLocalMultiplexing] =
    useState<MieruMultiplexing>(multiplexing)

  const handleChange = (value: MieruMultiplexing) => {
    setLocalMultiplexing(value)
    onChange(value)
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
      <Typography variant="h6">Mieru 多路复用配置</Typography>

      <FormControl fullWidth>
        <InputLabel>多路复用级别</InputLabel>
        <Select
          value={localMultiplexing}
          label="多路复用级别"
          onChange={(e) => handleChange(e.target.value as MieruMultiplexing)}
        >
          {multiplexingLevels.map((level) => (
            <MenuItem key={level.value} value={level.value}>
              <Box>
                <Typography variant="body2">{level.label}</Typography>
                <Typography variant="caption" color="text.secondary">
                  {level.description}
                </Typography>
              </Box>
            </MenuItem>
          ))}
        </Select>
      </FormControl>

      <Box sx={{ mt: 1 }}>
        <Typography variant="subtitle2" sx={{ mb: 1 }}>
          或使用单选按钮选择：
        </Typography>
        <RadioGroup
          value={localMultiplexing}
          onChange={(e) => handleChange(e.target.value as MieruMultiplexing)}
        >
          {multiplexingLevels.map((level) => (
            <FormControlLabel
              key={level.value}
              value={level.value}
              control={<Radio />}
              label={
                <Box>
                  <Typography variant="body2">{level.label}</Typography>
                  <Typography variant="caption" color="text.secondary">
                    {level.description}
                  </Typography>
                </Box>
              }
            />
          ))}
        </RadioGroup>
      </Box>
    </Box>
  )
}
