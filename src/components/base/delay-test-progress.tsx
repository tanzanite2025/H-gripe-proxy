/**
 * 延迟测试进度组件
 * 显示批量延迟测试的进度和取消按钮
 */

import { Box, Button, LinearProgress, Typography } from '@mui/material'
import { useEffect, useState } from 'react'

interface DelayTestProgressProps {
  total: number
  completed: number
  testing: boolean
  onCancel?: () => void
}

export const DelayTestProgress = ({
  total,
  completed,
  testing,
  onCancel,
}: DelayTestProgressProps) => {
  const [progress, setProgress] = useState(0)

  useEffect(() => {
    if (total > 0) {
      setProgress((completed / total) * 100)
    }
  }, [completed, total])

  if (!testing) {
    return null
  }

  return (
    <Box sx={{ width: '100%', mb: 2 }}>
      <Box
        sx={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          mb: 1,
        }}
      >
        <Typography variant="caption" color="text.secondary">
          正在测试延迟... {completed}/{total}
        </Typography>
        {onCancel && (
          <Button size="small" color="error" onClick={onCancel}>
            取消
          </Button>
        )}
      </Box>
      <LinearProgress variant="determinate" value={progress} />
    </Box>
  )
}

/**
 * 简单的延迟测试状态指示器
 */
export const DelayTestIndicator = ({ testing }: { testing: boolean }) => {
  if (!testing) {
    return null
  }

  return (
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
      <Box
        sx={{
          width: 8,
          height: 8,
          borderRadius: '50%',
          bgcolor: 'primary.main',
          animation: 'pulse 1.5s ease-in-out infinite',
          '@keyframes pulse': {
            '0%, 100%': {
              opacity: 1,
            },
            '50%': {
              opacity: 0.5,
            },
          },
        }}
      />
      <Typography variant="caption" color="text.secondary">
        测试中...
      </Typography>
    </Box>
  )
}
