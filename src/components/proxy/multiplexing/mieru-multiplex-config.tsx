import { useState } from 'react'

import { Radio, RadioGroup } from '@/components/tailwind/Radio'
import { Select } from '@/components/tailwind/Select'

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
    <div className="flex flex-col gap-4">
      <h3 className="text-lg font-semibold">Mieru 多路复用配置</h3>

      <Select
        value={localMultiplexing}
        label="多路复用级别"
        onChange={(e) => handleChange(e.target.value as MieruMultiplexing)}
        fullWidth
      >
        {multiplexingLevels.map((level) => (
          <option key={level.value} value={level.value}>
            {level.label} - {level.description}
          </option>
        ))}
      </Select>

      <div className="mt-2">
        <h4 className="text-sm font-medium mb-2">或使用单选按钮选择：</h4>
        <RadioGroup
          value={localMultiplexing}
          onChange={(e) => handleChange(e.target.value as MieruMultiplexing)}
        >
          {multiplexingLevels.map((level) => (
            <label key={level.value} className="flex items-start gap-2 mb-2">
              <Radio value={level.value} />
              <div>
                <div className="text-sm">{level.label}</div>
                <div className="text-xs text-gray-500">{level.description}</div>
              </div>
            </label>
          ))}
        </RadioGroup>
      </div>
    </div>
  )
}
