/**
 * 代理链帮助对话框
 * 提供使用说明、配置示例和最佳实践
 */

import {
  Close as CloseIcon,
  Help as HelpIcon,
  Info as InfoIcon,
  Warning as WarningIcon,
  CheckCircle as CheckIcon,
} from '@mui/icons-material'
import {
  Dialog,
  DialogTitle,
  DialogContent,
  IconButton,
  Box,
  Typography,
  Tabs,
  Tab,
  Alert,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  Divider,
  Paper,
  Chip,
} from '@mui/material'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

interface ProxyChainHelpDialogProps {
  open: boolean
  onClose: () => void
}

interface TabPanelProps {
  children?: React.ReactNode
  index: number
  value: number
}

const TabPanel = ({ children, value, index }: TabPanelProps) => {
  return (
    <div role="tabpanel" hidden={value !== index}>
      {value === index && <Box sx={{ py: 2 }}>{children}</Box>}
    </div>
  )
}

export const ProxyChainHelpDialog = ({
  open,
  onClose,
}: ProxyChainHelpDialogProps) => {
  const { t } = useTranslation()
  const [tabValue, setTabValue] = useState(0)

  const handleTabChange = (_event: React.SyntheticEvent, newValue: number) => {
    setTabValue(newValue)
  }

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <HelpIcon color="primary" />
            <Typography variant="h6">代理链使用指南</Typography>
          </Box>
          <IconButton onClick={onClose} size="small">
            <CloseIcon />
          </IconButton>
        </Box>
      </DialogTitle>

      <DialogContent>
        <Tabs value={tabValue} onChange={handleTabChange} sx={{ borderBottom: 1, borderColor: 'divider' }}>
          <Tab label="什么是代理链" />
          <Tab label="如何配置" />
          <Tab label="配置示例" />
          <Tab label="最佳实践" />
          <Tab label="常见问题" />
        </Tabs>

        {/* Tab 1: 什么是代理链 */}
        <TabPanel value={tabValue} index={0}>
          <Typography variant="h6" gutterBottom>
            什么是代理链？
          </Typography>
          <Typography variant="body2" sx={{ mb: 2 }}>
            代理链（Proxy Chain）是指将多个代理服务器串联起来，流量依次通过每个代理节点，最终到达目标服务器。
          </Typography>

          <Alert severity="info" sx={{ mb: 2 }}>
            <Typography variant="body2">
              <strong>工作原理：</strong>
              <br />
              你的设备 → 入口节点 → 中间节点 → 出口节点 → 目标网站
            </Typography>
          </Alert>

          <Typography variant="subtitle2" gutterBottom sx={{ mt: 2 }}>
            代理链的优势
          </Typography>
          <List dense>
            <ListItem>
              <ListItemIcon>
                <CheckIcon color="success" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="增强隐私保护"
                secondary="多层代理使得追踪真实 IP 更加困难"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <CheckIcon color="success" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="绕过地区限制"
                secondary="通过不同地区的节点访问特定内容"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <CheckIcon color="success" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="负载均衡"
                secondary="分散流量到多个节点，避免单点过载"
              />
            </ListItem>
          </List>

          <Typography variant="subtitle2" gutterBottom sx={{ mt: 2 }}>
            代理链的劣势
          </Typography>
          <List dense>
            <ListItem>
              <ListItemIcon>
                <WarningIcon color="warning" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="延迟增加"
                secondary="每增加一个节点，延迟会累加"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <WarningIcon color="warning" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="速度降低"
                secondary="受限于链中最慢的节点"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <WarningIcon color="warning" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="稳定性降低"
                secondary="任何一个节点故障都会导致整条链路失败"
              />
            </ListItem>
          </List>
        </TabPanel>

        {/* Tab 2: 如何配置 */}
        <TabPanel value={tabValue} index={1}>
          <Typography variant="h6" gutterBottom>
            如何配置代理链？
          </Typography>

          <Alert severity="info" sx={{ mb: 2 }}>
            代理链至少需要 <strong>2 个节点</strong>（入口节点 + 出口节点）
          </Alert>

          <Typography variant="subtitle2" gutterBottom>
            配置步骤
          </Typography>
          <List>
            <ListItem>
              <ListItemIcon>
                <Chip label="1" size="small" color="primary" />
              </ListItemIcon>
              <ListItemText
                primary="启用链式模式"
                secondary="点击右上角的 '链式模式' 开关"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <Chip label="2" size="small" color="primary" />
              </ListItemIcon>
              <ListItemText
                primary="选择代理组"
                secondary="在规则模式下，选择要使用的代理组"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <Chip label="3" size="small" color="primary" />
              </ListItemIcon>
              <ListItemText
                primary="添加节点"
                secondary="依次点击节点，按顺序添加到代理链中"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <Chip label="4" size="small" color="primary" />
              </ListItemIcon>
              <ListItemText
                primary="调整顺序"
                secondary="拖拽节点卡片可以调整顺序"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <Chip label="5" size="small" color="primary" />
              </ListItemIcon>
              <ListItemText
                primary="连接代理链"
                secondary="点击 '连接' 按钮启用代理链"
              />
            </ListItem>
          </List>

          <Divider sx={{ my: 2 }} />

          <Typography variant="subtitle2" gutterBottom>
            节点角色说明
          </Typography>
          <Paper variant="outlined" sx={{ p: 2, mb: 1 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
              <Chip label="入口节点" size="small" color="success" />
              <Typography variant="body2">
                第一个节点，你的设备直接连接到这个节点
              </Typography>
            </Box>
            <Typography variant="caption" color="text.secondary">
              建议：选择延迟低、稳定性高的节点
            </Typography>
          </Paper>

          <Paper variant="outlined" sx={{ p: 2, mb: 1 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
              <Chip label="中间节点" size="small" color="primary" />
              <Typography variant="body2">
                中间的节点，用于转发流量
              </Typography>
            </Box>
            <Typography variant="caption" color="text.secondary">
              建议：可选，用于增强隐私或绕过特定限制
            </Typography>
          </Paper>

          <Paper variant="outlined" sx={{ p: 2 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
              <Chip label="出口节点" size="small" color="warning" />
              <Typography variant="body2">
                最后一个节点，目标网站看到的是这个节点的 IP
              </Typography>
            </Box>
            <Typography variant="caption" color="text.secondary">
              建议：选择目标地区的节点，以获得最佳访问效果
            </Typography>
          </Paper>
        </TabPanel>

        {/* Tab 3: 配置示例 */}
        <TabPanel value={tabValue} index={2}>
          <Typography variant="h6" gutterBottom>
            配置示例
          </Typography>

          <Typography variant="subtitle2" gutterBottom sx={{ mt: 2 }}>
            示例 1: 双重代理（基础）
          </Typography>
          <Paper variant="outlined" sx={{ p: 2, mb: 2 }}>
            <Typography variant="body2" gutterBottom>
              <strong>场景：</strong>访问国外网站，增强隐私保护
            </Typography>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, my: 1 }}>
              <Chip label="入口" size="small" color="success" />
              <Typography variant="body2">香港节点（低延迟）</Typography>
            </Box>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, my: 1 }}>
              <Chip label="出口" size="small" color="warning" />
              <Typography variant="body2">美国节点（目标地区）</Typography>
            </Box>
            <Alert severity="info" sx={{ mt: 1 }}>
              <Typography variant="caption">
                预估延迟: 150-250ms | 适用场景: 日常使用
              </Typography>
            </Alert>
          </Paper>

          <Typography variant="subtitle2" gutterBottom>
            示例 2: 三重代理（增强隐私）
          </Typography>
          <Paper variant="outlined" sx={{ p: 2, mb: 2 }}>
            <Typography variant="body2" gutterBottom>
              <strong>场景：</strong>需要最强隐私保护
            </Typography>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, my: 1 }}>
              <Chip label="入口" size="small" color="success" />
              <Typography variant="body2">香港节点</Typography>
            </Box>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, my: 1 }}>
              <Chip label="中间" size="small" color="primary" />
              <Typography variant="body2">新加坡节点</Typography>
            </Box>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, my: 1 }}>
              <Chip label="出口" size="small" color="warning" />
              <Typography variant="body2">美国节点</Typography>
            </Box>
            <Alert severity="warning" sx={{ mt: 1 }}>
              <Typography variant="caption">
                预估延迟: 250-400ms | 适用场景: 高隐私需求
              </Typography>
            </Alert>
          </Paper>

          <Typography variant="subtitle2" gutterBottom>
            示例 3: 地区链（绕过限制）
          </Typography>
          <Paper variant="outlined" sx={{ p: 2, mb: 2 }}>
            <Typography variant="body2" gutterBottom>
              <strong>场景：</strong>访问特定地区限制的内容
            </Typography>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, my: 1 }}>
              <Chip label="入口" size="small" color="success" />
              <Typography variant="body2">日本节点（低延迟）</Typography>
            </Box>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, my: 1 }}>
              <Chip label="出口" size="small" color="warning" />
              <Typography variant="body2">台湾节点（目标地区）</Typography>
            </Box>
            <Alert severity="info" sx={{ mt: 1 }}>
              <Typography variant="caption">
                预估延迟: 100-180ms | 适用场景: 访问台湾限定内容
              </Typography>
            </Alert>
          </Paper>
        </TabPanel>

        {/* Tab 4: 最佳实践 */}
        <TabPanel value={tabValue} index={3}>
          <Typography variant="h6" gutterBottom>
            最佳实践
          </Typography>

          <Typography variant="subtitle2" gutterBottom sx={{ mt: 2 }}>
            节点选择建议
          </Typography>
          <List dense>
            <ListItem>
              <ListItemIcon>
                <InfoIcon color="info" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="入口节点：选择延迟最低的节点"
                secondary="通常选择地理位置最近的节点"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <InfoIcon color="info" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="出口节点：选择目标地区的节点"
                secondary="根据要访问的内容选择合适的地区"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <InfoIcon color="info" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="避免使用过多节点"
                secondary="2-3 个节点通常足够，过多会严重影响速度"
              />
            </ListItem>
          </List>

          <Divider sx={{ my: 2 }} />

          <Typography variant="subtitle2" gutterBottom>
            性能优化建议
          </Typography>
          <List dense>
            <ListItem>
              <ListItemIcon>
                <CheckIcon color="success" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="定期测试节点延迟"
                secondary="使用延迟测试功能，选择延迟低的节点"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <CheckIcon color="success" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="避免跨大洲跳转"
                secondary="尽量选择地理位置相近的节点"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <CheckIcon color="success" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="使用高速协议"
                secondary="优先选择 Trojan、VMess 等高速协议"
              />
            </ListItem>
          </List>

          <Divider sx={{ my: 2 }} />

          <Typography variant="subtitle2" gutterBottom>
            安全建议
          </Typography>
          <List dense>
            <ListItem>
              <ListItemIcon>
                <WarningIcon color="warning" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="不要使用不可信的节点"
                secondary="确保所有节点都来自可信的提供商"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <WarningIcon color="warning" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="定期更换代理链配置"
                secondary="避免长期使用相同的代理链"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <WarningIcon color="warning" fontSize="small" />
              </ListItemIcon>
              <ListItemText
                primary="配合 DNS 加密使用"
                secondary="启用 DoH/DoT 防止 DNS 泄漏"
              />
            </ListItem>
          </List>
        </TabPanel>

        {/* Tab 5: 常见问题 */}
        <TabPanel value={tabValue} index={4}>
          <Typography variant="h6" gutterBottom>
            常见问题
          </Typography>

          <Paper variant="outlined" sx={{ p: 2, mb: 2 }}>
            <Typography variant="subtitle2" gutterBottom>
              Q: 代理链连接失败怎么办？
            </Typography>
            <Typography variant="body2" color="text.secondary">
              A: 检查以下几点：
            </Typography>
            <List dense>
              <ListItem>
                <Typography variant="body2">• 确保所有节点都可用（测试延迟）</Typography>
              </ListItem>
              <ListItem>
                <Typography variant="body2">• 确保至少有 2 个节点</Typography>
              </ListItem>
              <ListItem>
                <Typography variant="body2">• 尝试更换节点顺序</Typography>
              </ListItem>
              <ListItem>
                <Typography variant="body2">• 检查节点协议是否兼容</Typography>
              </ListItem>
            </List>
          </Paper>

          <Paper variant="outlined" sx={{ p: 2, mb: 2 }}>
            <Typography variant="subtitle2" gutterBottom>
              Q: 代理链速度很慢怎么办？
            </Typography>
            <Typography variant="body2" color="text.secondary">
              A: 尝试以下优化：
            </Typography>
            <List dense>
              <ListItem>
                <Typography variant="body2">• 减少节点数量（2 个节点通常足够）</Typography>
              </ListItem>
              <ListItem>
                <Typography variant="body2">• 选择延迟低的节点</Typography>
              </ListItem>
              <ListItem>
                <Typography variant="body2">• 避免跨大洲跳转</Typography>
              </ListItem>
              <ListItem>
                <Typography variant="body2">• 使用高速协议节点</Typography>
              </ListItem>
            </List>
          </Paper>

          <Paper variant="outlined" sx={{ p: 2, mb: 2 }}>
            <Typography variant="subtitle2" gutterBottom>
              Q: 代理链和普通代理有什么区别？
            </Typography>
            <Typography variant="body2" color="text.secondary">
              A: 主要区别：
            </Typography>
            <List dense>
              <ListItem>
                <Typography variant="body2">
                  • 普通代理：设备 → 代理 → 目标网站
                </Typography>
              </ListItem>
              <ListItem>
                <Typography variant="body2">
                  • 代理链：设备 → 代理1 → 代理2 → ... → 目标网站
                </Typography>
              </ListItem>
              <ListItem>
                <Typography variant="body2">
                  • 代理链提供更强的隐私保护，但速度较慢
                </Typography>
              </ListItem>
            </List>
          </Paper>

          <Paper variant="outlined" sx={{ p: 2, mb: 2 }}>
            <Typography variant="subtitle2" gutterBottom>
              Q: 如何保存代理链配置？
            </Typography>
            <Typography variant="body2" color="text.secondary">
              A: 代理链配置会自动保存到浏览器的 localStorage 中，下次打开时会自动恢复。
              如果需要在不同设备间同步配置，可以手动记录节点顺序。
            </Typography>
          </Paper>

          <Paper variant="outlined" sx={{ p: 2 }}>
            <Typography variant="subtitle2" gutterBottom>
              Q: 代理链适合什么场景？
            </Typography>
            <Typography variant="body2" color="text.secondary">
              A: 适合场景：
            </Typography>
            <List dense>
              <ListItem>
                <Typography variant="body2">• 需要增强隐私保护</Typography>
              </ListItem>
              <ListItem>
                <Typography variant="body2">• 需要绕过多层地区限制</Typography>
              </ListItem>
              <ListItem>
                <Typography variant="body2">• 需要隐藏真实 IP</Typography>
              </ListItem>
            </List>
            <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
              不适合场景：
            </Typography>
            <List dense>
              <ListItem>
                <Typography variant="body2">• 需要高速下载/上传</Typography>
              </ListItem>
              <ListItem>
                <Typography variant="body2">• 需要低延迟（游戏、视频通话）</Typography>
              </ListItem>
              <ListItem>
                <Typography variant="body2">• 日常轻度使用</Typography>
              </ListItem>
            </List>
          </Paper>
        </TabPanel>
      </DialogContent>
    </Dialog>
  )
}
