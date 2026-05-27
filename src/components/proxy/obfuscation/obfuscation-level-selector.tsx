import { useState } from 'react'

import { Chip } from '@/components/tailwind/Chip'
import {
  getAllObfuscationStrategies,
  type ObfuscationLevel,
  type ObfuscationStrategy,
} from '@/services/obfuscation'

interface ObfuscationLevelSelectorProps {
  currentLevel: ObfuscationLevel
  onChange: (level: ObfuscationLevel) => void
}

export function ObfuscationLevelSelector({
  currentLevel,
  onChange,
}: ObfuscationLevelSelectorProps) {
  const [selectedLevel, setSelectedLevel] =
    useState<ObfuscationLevel>(currentLevel)
  const strategies = getAllObfuscationStrategies()

  const handleSelect = (level: ObfuscationLevel) => {
    setSelectedLevel(level)
    onChange(level)
  }

  const getPerformanceImpact = (level: ObfuscationLevel): string => {
    const impacts: Record<ObfuscationLevel, string> = {
      none: '无影响',
      low: '极小',
      medium: '较小',
      high: '中等',
      paranoid: '较大',
    }
    return impacts[level]
  }

  const getRecommendation = (level: ObfuscationLevel): string | null => {
    if (level === 'medium') return '推荐'
    if (level === 'none') return '不推荐'
    return null
  }

  return (
    <div>
      <h3 className="text-lg font-semibold mb-4">选择混淆级别</h3>

      <div className="flex flex-col gap-4">
        {strategies.map((strategy: ObfuscationStrategy) => {
          const isSelected = selectedLevel === strategy.level
          const recommendation = getRecommendation(strategy.level)

          return (
            <div
              key={strategy.level}
              className={`rounded-lg border-2 cursor-pointer transition-colors ${
                isSelected
                  ? 'border-primary bg-primary/5'
                  : 'border-gray-200 dark:border-gray-700 hover:border-primary/50'
              }`}
              onClick={() => handleSelect(strategy.level)}
            >
              <div className="p-4">
                <div className="flex items-center justify-between mb-2">
                  <h4 className="text-lg font-semibold">{strategy.name}</h4>
                  <div className="flex gap-2">
                    {recommendation && (
                      <Chip
                        label={recommendation}
                        color={
                          recommendation === '推荐' ? 'success' : 'warning'
                        }
                        size="small"
                      />
                    )}
                    {isSelected && (
                      <Chip label="当前" color="primary" size="small" />
                    )}
                  </div>
                </div>

                <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
                  {strategy.description}
                </p>

                <div className="flex gap-2 flex-wrap">
                  <Chip
                    label={`性能影响: ${getPerformanceImpact(strategy.level)}`}
                    size="small"
                    variant="outlined"
                  />
                  {strategy.features.trafficObfuscation && (
                    <Chip label="流量混淆" size="small" variant="outlined" />
                  )}
                  {strategy.features.protocolObfuscation && (
                    <Chip label="协议混淆" size="small" variant="outlined" />
                  )}
                  {strategy.features.tlsFingerprintRandomization && (
                    <Chip label="TLS指纹" size="small" variant="outlined" />
                  )}
                </div>
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
