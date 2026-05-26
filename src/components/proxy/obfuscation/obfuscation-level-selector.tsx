import {
  Box,
  Card,
  CardActionArea,
  CardContent,
  Chip,
  Typography,
} from '@mui/material'
import { useState } from 'react'

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
    <Box>
      <Typography variant="h6" gutterBottom>
        选择混淆级别
      </Typography>

      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {strategies.map((strategy: ObfuscationStrategy) => {
          const isSelected = selectedLevel === strategy.level
          const recommendation = getRecommendation(strategy.level)

          return (
            <Card
              key={strategy.level}
              variant={isSelected ? 'elevation' : 'outlined'}
              sx={{
                borderColor: isSelected ? 'primary.main' : 'divider',
                borderWidth: isSelected ? 2 : 1,
              }}
            >
              <CardActionArea onClick={() => handleSelect(strategy.level)}>
                <CardContent>
                  <Box
                    sx={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      mb: 1,
                    }}
                  >
                    <Typography variant="h6">{strategy.name}</Typography>
                    <Box sx={{ display: 'flex', gap: 1 }}>
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
                    </Box>
                  </Box>

                  <Typography
                    variant="body2"
                    color="text.secondary"
                    gutterBottom
                  >
                    {strategy.description}
                  </Typography>

                  <Box sx={{ mt: 2, display: 'flex', gap: 1, flexWrap: 'wrap' }}>
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
                  </Box>
                </CardContent>
              </CardActionArea>
            </Card>
          )
        })}
      </Box>
    </Box>
  )
}
