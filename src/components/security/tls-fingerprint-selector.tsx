/**
 * TLS 指纹选择器组件
 */

import {
  Box,
  Button,
  Card,
  CardContent,
  Chip,
  Paper,
  Stack,
  Typography,
} from '@mui/material'
import {
  CheckCircleOutlined,
  FingerprintOutlined,
  InfoOutlined,
} from '@mui/icons-material'
import { useEffect, useState } from 'react'

import {
  type TlsFingerprint,
  tlsFingerprintClear,
  tlsFingerprintGetAll,
  tlsFingerprintGetCurrent,
  tlsFingerprintSetByName,
} from '@/services/tls-fingerprint'
import { showNotice } from '@/services/notice-service'

export default function TlsFingerprintSelector() {
  const [fingerprints, setFingerprints] = useState<TlsFingerprint[]>([])
  const [currentFingerprint, setCurrentFingerprint] =
    useState<TlsFingerprint | null>(null)
  const [loading, setLoading] = useState(false)

  // 加载指纹列表
  useEffect(() => {
    loadFingerprints()
    loadCurrentFingerprint()
  }, [])

  const loadFingerprints = async () => {
    try {
      const fps = await tlsFingerprintGetAll()
      setFingerprints(fps)
    } catch (error) {
      showNotice.error(`加载指纹列表失败: ${error}`)
    }
  }

  const loadCurrentFingerprint = async () => {
    try {
      const fp = await tlsFingerprintGetCurrent()
      setCurrentFingerprint(fp)
    } catch (error) {
      console.error('加载当前指纹失败:', error)
    }
  }

  // 选择指纹
  const handleSelectFingerprint = async (name: string) => {
    try {
      setLoading(true)
      await tlsFingerprintSetByName(name)
      await loadCurrentFingerprint()
      showNotice.success(`已切换到 ${name}`)
    } catch (error) {
      showNotice.error(`切换指纹失败: ${error}`)
    } finally {
      setLoading(false)
    }
  }

  // 清除指纹
  const handleClearFingerprint = async () => {
    try {
      setLoading(true)
      await tlsFingerprintClear()
      setCurrentFingerprint(null)
      showNotice.success('已清除 TLS 指纹伪装')
    } catch (error) {
      showNotice.error(`清除失败: ${error}`)
    } finally {
      setLoading(false)
    }
  }

  // 获取指纹图标
  const getFingerprintIcon = (name: string) => {
    if (name.includes('Chrome')) return '🌐'
    if (name.includes('Firefox')) return '🦊'
    if (name.includes('Safari')) return '🧭'
    if (name.includes('Genshin')) return '🎮'
    return '🔒'
  }

  return (
    <Box sx={{ p: 3 }}>
      <Stack spacing={3}>
        {/* 标题 */}
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <FingerprintOutlined color="primary" />
          <Typography variant="h6">TLS 指纹伪装（Parrot Mode）</Typography>
        </Box>

        {/* 说明 */}
        <Paper sx={{ p: 2, bgcolor: 'info.main', color: 'info.contrastText' }}>
          <Box sx={{ display: 'flex', alignItems: 'flex-start', gap: 1 }}>
            <InfoOutlined />
            <Box>
              <Typography variant="body2" sx={{ fontWeight: 600 }}>
                ALPN 真实指纹复刻
              </Typography>
              <Typography variant="caption">
                100%
                复刻真实浏览器/应用的 TLS 指纹（JA3/JA4），让翻墙流量在统计学上和普通人刷网页、打游戏毫无二致。
              </Typography>
            </Box>
          </Box>
        </Paper>

        {/* 当前指纹 */}
        {currentFingerprint && (
          <Paper sx={{ p: 2, bgcolor: 'success.main', color: 'success.contrastText' }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
              <CheckCircleOutlined />
              <Typography variant="subtitle2">当前使用指纹</Typography>
            </Box>
            <Typography variant="h6">
              {getFingerprintIcon(currentFingerprint.name)}{' '}
              {currentFingerprint.name}
            </Typography>
            <Typography variant="caption">
              {currentFingerprint.description}
            </Typography>
            <Box sx={{ mt: 1 }}>
              <Button
                variant="outlined"
                size="small"
                onClick={handleClearFingerprint}
                disabled={loading}
                sx={{ color: 'inherit', borderColor: 'inherit' }}
              >
                清除伪装
              </Button>
            </Box>
          </Paper>
        )}

        {/* 指纹列表 */}
        <Typography variant="subtitle2">选择 TLS 指纹</Typography>
        <Box
          sx={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fill, minmax(300px, 1fr))',
            gap: 2,
          }}
        >
          {fingerprints.map((fp) => {
            const isActive = currentFingerprint?.name === fp.name

            return (
              <Card
                key={fp.name}
                sx={{
                  cursor: 'pointer',
                  border: isActive ? 2 : 1,
                  borderColor: isActive ? 'primary.main' : 'divider',
                  transition: 'all 0.2s',
                  '&:hover': {
                    borderColor: 'primary.main',
                    transform: 'translateY(-2px)',
                    boxShadow: 3,
                  },
                }}
                onClick={() => handleSelectFingerprint(fp.name)}
              >
                <CardContent>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
                    <Typography variant="h4">
                      {getFingerprintIcon(fp.name)}
                    </Typography>
                    <Box sx={{ flex: 1 }}>
                      <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
                        {fp.name}
                      </Typography>
                      {isActive && (
                        <Chip
                          label="使用中"
                          size="small"
                          color="primary"
                          sx={{ height: 20 }}
                        />
                      )}
                    </Box>
                  </Box>

                  <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                    {fp.description}
                  </Typography>

                  <Stack spacing={1}>
                    <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
                      <Chip label={fp.tls_version} size="small" />
                      <Chip
                        label={`${fp.cipher_suites.length} 密码套件`}
                        size="small"
                      />
                    </Box>

                    <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
                      {fp.alpn_protocols.map((alpn) => (
                        <Chip
                          key={alpn}
                          label={alpn}
                          size="small"
                          variant="outlined"
                        />
                      ))}
                    </Box>

                    <Typography
                      variant="caption"
                      color="text.secondary"
                      sx={{
                        fontFamily: 'monospace',
                        fontSize: '0.7rem',
                        wordBreak: 'break-all',
                      }}
                    >
                      JA3: {fp.ja3_fingerprint.substring(0, 40)}...
                    </Typography>
                  </Stack>
                </CardContent>
              </Card>
            )
          })}
        </Box>
      </Stack>
    </Box>
  )
}
