/**
 * 代理链帮助对话框
 * 提供使用说明、配置示例和最佳实践
 */

import { X as CloseIcon, HelpCircle as HelpIcon, Info as InfoIcon, AlertTriangle as WarningIcon, CheckCircle as CheckIcon } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Alert } from '@/components/tailwind/Alert'
import { Chip } from '@/components/tailwind/Chip'
import { Dialog, DialogContent, DialogTitle } from '@/components/tailwind/Dialog'
import { IconButton } from '@/components/tailwind/IconButton'
import { List, ListItem, ListItemIcon, ListItemText } from '@/components/tailwind/List'
import { Paper } from '@/components/tailwind/Paper'
import { Tab, Tabs } from '@/components/tailwind/Tabs'

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
      {value === index && <div className="py-4">{children}</div>}
    </div>
  )
}

export const ProxyChainHelpDialog = ({
  open,
  onClose,
}: ProxyChainHelpDialogProps) => {
  const { t } = useTranslation()
  const [tabValue, setTabValue] = useState(0)

  const handleTabChange = (_event: React.SyntheticEvent, newValue: string | number) => {
    setTabValue(newValue as number)
  }

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <HelpIcon className="text-primary" />
            <h6 className="text-lg font-semibold">代理链使用指南</h6>
          </div>
          <IconButton onClick={onClose} size="small">
            <CloseIcon />
          </IconButton>
        </div>
      </DialogTitle>

      <DialogContent>
        <Tabs value={tabValue} onChange={handleTabChange} className="border-b border-divider">
          <Tab label="什么是代理链" />
          <Tab label="如何配置" />
          <Tab label="配置示例" />
          <Tab label="最佳实践" />
          <Tab label="常见问题" />
        </Tabs>

        {/* Tab 1: 什么是代理链 */}
        <TabPanel value={tabValue} index={0}>
          <h6 className="text-base font-semibold mb-2">
            什么是代理链？
          </h6>
          <p className="text-sm mb-4">
            代理链（Proxy Chain）是指将多个代理服务器串联起来，流量依次通过每个代理节点，最终到达目标服务器。
          </p>

          <Alert severity="info" className="mb-4">
            <p className="text-sm">
              <strong>工作原理：</strong>
              <br />
              你的设备 → 入口节点 → 中间节点 → 出口节点 → 目标网站
            </p>
          </Alert>

          <h6 className="text-sm font-semibold mb-2 mt-4">
            代理链的优势
          </h6>
          <List>
            <ListItem>
              <ListItemIcon>
                <CheckIcon className="text-success text-sm" />
              </ListItemIcon>
              <ListItemText
                primary="增强隐私保护"
                secondary="多层代理使得追踪真实 IP 更加困难"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <CheckIcon className="text-success text-sm" />
              </ListItemIcon>
              <ListItemText
                primary="绕过地区限制"
                secondary="通过不同地区的节点访问特定内容"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <CheckIcon className="text-success text-sm" />
              </ListItemIcon>
              <ListItemText
                primary="负载均衡"
                secondary="分散流量到多个节点，避免单点过载"
              />
            </ListItem>
          </List>

          <h6 className="text-sm font-semibold mb-2 mt-4">
            代理链的劣势
          </h6>
          <List>
            <ListItem>
              <ListItemIcon>
                <WarningIcon className="text-warning text-sm" />
              </ListItemIcon>
              <ListItemText
                primary="延迟增加"
                secondary="每增加一个节点，延迟会累加"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <WarningIcon className="text-warning text-sm" />
              </ListItemIcon>
              <ListItemText
                primary="速度降低"
                secondary="受限于链中最慢的节点"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <WarningIcon className="text-warning text-sm" />
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
          <h6 className="text-base font-semibold mb-2">
            如何配置代理链？
          </h6>

          <Alert severity="info" className="mb-4">
            代理链至少需要 <strong>2 个节点</strong>（入口节点 + 出口节点）
          </Alert>

          <h6 className="text-sm font-semibold mb-2">
            配置步骤
          </h6>
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

          <div className="my-4 border-t border-divider" />

          <h6 className="text-sm font-semibold mb-2">
            节点角色说明
          </h6>
          <Paper variant="outlined" className="p-4 mb-2">
            <div className="flex items-center gap-2 mb-2">
              <Chip label="入口节点" size="small" color="success" />
              <p className="text-sm">
                第一个节点，你的设备直接连接到这个节点
              </p>
            </div>
            <p className="text-xs text-text-secondary">
              建议：选择延迟低、稳定性高的节点
            </p>
          </Paper>

          <Paper variant="outlined" className="p-4 mb-2">
            <div className="flex items-center gap-2 mb-2">
              <Chip label="中间节点" size="small" color="primary" />
              <p className="text-sm">
                中间的节点，用于转发流量
              </p>
            </div>
            <p className="text-xs text-text-secondary">
              建议：可选，用于增强隐私或绕过特定限制
            </p>
          </Paper>

          <Paper variant="outlined" className="p-4">
            <div className="flex items-center gap-2 mb-2">
              <Chip label="出口节点" size="small" color="warning" />
              <p className="text-sm">
                最后一个节点，目标网站看到的是这个节点的 IP
              </p>
            </div>
            <p className="text-xs text-text-secondary">
              建议：选择目标地区的节点，以获得最佳访问效果
            </p>
          </Paper>
        </TabPanel>

        {/* Tab 3: 配置示例 */}
        <TabPanel value={tabValue} index={2}>
          <h6 className="text-base font-semibold mb-2">
            配置示例
          </h6>

          <h6 className="text-sm font-semibold mb-2 mt-4">
            示例 1: 双重代理（基础）
          </h6>
          <Paper variant="outlined" className="p-4 mb-4">
            <p className="text-sm mb-2">
              <strong>场景：</strong>访问国外网站，增强隐私保护
            </p>
            <div className="flex items-center gap-2 my-2">
              <Chip label="入口" size="small" color="success" />
              <p className="text-sm">香港节点（低延迟）</p>
            </div>
            <div className="flex items-center gap-2 my-2">
              <Chip label="出口" size="small" color="warning" />
              <p className="text-sm">美国节点（目标地区）</p>
            </div>
            <Alert severity="info" className="mt-2">
              <p className="text-xs">
                预估延迟: 150-250ms | 适用场景: 日常使用
              </p>
            </Alert>
          </Paper>

          <h6 className="text-sm font-semibold mb-2">
            示例 2: 三重代理（增强隐私）
          </h6>
          <Paper variant="outlined" className="p-4 mb-4">
            <p className="text-sm mb-2">
              <strong>场景：</strong>需要最强隐私保护
            </p>
            <div className="flex items-center gap-2 my-2">
              <Chip label="入口" size="small" color="success" />
              <p className="text-sm">香港节点</p>
            </div>
            <div className="flex items-center gap-2 my-2">
              <Chip label="中间" size="small" color="primary" />
              <p className="text-sm">新加坡节点</p>
            </div>
            <div className="flex items-center gap-2 my-2">
              <Chip label="出口" size="small" color="warning" />
              <p className="text-sm">美国节点</p>
            </div>
            <Alert severity="warning" className="mt-2">
              <p className="text-xs">
                预估延迟: 250-400ms | 适用场景: 高隐私需求
              </p>
            </Alert>
          </Paper>

          <h6 className="text-sm font-semibold mb-2">
            示例 3: 地区链（绕过限制）
          </h6>
          <Paper variant="outlined" className="p-4 mb-4">
            <p className="text-sm mb-2">
              <strong>场景：</strong>访问特定地区限制的内容
            </p>
            <div className="flex items-center gap-2 my-2">
              <Chip label="入口" size="small" color="success" />
              <p className="text-sm">日本节点（低延迟）</p>
            </div>
            <div className="flex items-center gap-2 my-2">
              <Chip label="出口" size="small" color="warning" />
              <p className="text-sm">台湾节点（目标地区）</p>
            </div>
            <Alert severity="info" className="mt-2">
              <p className="text-xs">
                预估延迟: 100-180ms | 适用场景: 访问台湾限定内容
              </p>
            </Alert>
          </Paper>
        </TabPanel>

        {/* Tab 4: 最佳实践 */}
        <TabPanel value={tabValue} index={3}>
          <h6 className="text-base font-semibold mb-2">
            最佳实践
          </h6>

          <h6 className="text-sm font-semibold mb-2 mt-4">
            节点选择建议
          </h6>
          <List>
            <ListItem>
              <ListItemIcon>
                <InfoIcon className="text-info text-sm" />
              </ListItemIcon>
              <ListItemText
                primary="入口节点：选择延迟最低的节点"
                secondary="通常选择地理位置最近的节点"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <InfoIcon className="text-info text-sm" />
              </ListItemIcon>
              <ListItemText
                primary="出口节点：选择目标地区的节点"
                secondary="根据要访问的内容选择合适的地区"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <InfoIcon className="text-info text-sm" />
              </ListItemIcon>
              <ListItemText
                primary="避免使用过多节点"
                secondary="2-3 个节点通常足够，过多会严重影响速度"
              />
            </ListItem>
          </List>

          <div className="my-4 border-t border-divider" />

          <h6 className="text-sm font-semibold mb-2">
            性能优化建议
          </h6>
          <List>
            <ListItem>
              <ListItemIcon>
                <CheckIcon className="text-success text-sm" />
              </ListItemIcon>
              <ListItemText
                primary="定期测试节点延迟"
                secondary="使用延迟测试功能，选择延迟低的节点"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <CheckIcon className="text-success text-sm" />
              </ListItemIcon>
              <ListItemText
                primary="避免跨大洲跳转"
                secondary="尽量选择地理位置相近的节点"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <CheckIcon className="text-success text-sm" />
              </ListItemIcon>
              <ListItemText
                primary="使用高速协议"
                secondary="优先选择 Trojan、VMess 等高速协议"
              />
            </ListItem>
          </List>

          <div className="my-4 border-t border-divider" />

          <h6 className="text-sm font-semibold mb-2">
            安全建议
          </h6>
          <List>
            <ListItem>
              <ListItemIcon>
                <WarningIcon className="text-warning text-sm" />
              </ListItemIcon>
              <ListItemText
                primary="不要使用不可信的节点"
                secondary="确保所有节点都来自可信的提供商"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <WarningIcon className="text-warning text-sm" />
              </ListItemIcon>
              <ListItemText
                primary="定期更换代理链配置"
                secondary="避免长期使用相同的代理链"
              />
            </ListItem>
            <ListItem>
              <ListItemIcon>
                <WarningIcon className="text-warning text-sm" />
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
          <h6 className="text-base font-semibold mb-2">
            常见问题
          </h6>

          <Paper variant="outlined" className="p-4 mb-4">
            <h6 className="text-sm font-semibold mb-2">
              Q: 代理链连接失败怎么办？
            </h6>
            <p className="text-sm text-text-secondary">
              A: 检查以下几点：
            </p>
            <List>
              <ListItem>
                <p className="text-sm">• 确保所有节点都可用（测试延迟）</p>
              </ListItem>
              <ListItem>
                <p className="text-sm">• 确保至少有 2 个节点</p>
              </ListItem>
              <ListItem>
                <p className="text-sm">• 尝试更换节点顺序</p>
              </ListItem>
              <ListItem>
                <p className="text-sm">• 检查节点协议是否兼容</p>
              </ListItem>
            </List>
          </Paper>

          <Paper variant="outlined" className="p-4 mb-4">
            <h6 className="text-sm font-semibold mb-2">
              Q: 代理链速度很慢怎么办？
            </h6>
            <p className="text-sm text-text-secondary">
              A: 尝试以下优化：
            </p>
            <List>
              <ListItem>
                <p className="text-sm">• 减少节点数量，2 个节点通常足够</p>
              </ListItem>
              <ListItem>
                <p className="text-sm">• 选择延迟低的节点</p>
              </ListItem>
              <ListItem>
                <p className="text-sm">• 避免跨大洲跳转</p>
              </ListItem>
              <ListItem>
                <p className="text-sm">• 使用高速协议节点</p>
              </ListItem>
            </List>
          </Paper>

          <Paper variant="outlined" className="p-4 mb-4">
            <h6 className="text-sm font-semibold mb-2">
              Q: 代理链和普通代理有什么区别？
            </h6>
            <p className="text-sm text-text-secondary">
              A: 主要区别：
            </p>
            <List>
              <ListItem>
                <p className="text-sm">
                  • 普通代理：设备 → 代理 → 目标网站
                </p>
              </ListItem>
              <ListItem>
                <p className="text-sm">
                  • 代理链：设备 → 代理1 → 代理2 → ... → 目标网站
                </p>
              </ListItem>
              <ListItem>
                <p className="text-sm">
                  • 代理链提供更强的隐私保护，但速度较慢
                </p>
              </ListItem>
            </List>
          </Paper>

          <Paper variant="outlined" className="p-4 mb-4">
            <h6 className="text-sm font-semibold mb-2">
              Q: 如何保存代理链配置？
            </h6>
            <p className="text-sm text-text-secondary">
              A: 代理链配置会自动保存到浏览器的 localStorage 中，下次打开时会自动恢复。
              如果需要在不同设备间同步配置，可以手动记录节点顺序。
            </p>
          </Paper>

          <Paper variant="outlined" className="p-4">
            <h6 className="text-sm font-semibold mb-2">
              Q: 代理链适合什么场景？
            </h6>
            <p className="text-sm text-text-secondary">
              A: 适合场景：
            </p>
            <List>
              <ListItem>
                <p className="text-sm">• 需要增强隐私保护</p>
              </ListItem>
              <ListItem>
                <p className="text-sm">• 需要绕过多层地区限制</p>
              </ListItem>
              <ListItem>
                <p className="text-sm">• 需要隐藏真实 IP</p>
              </ListItem>
            </List>
            <p className="text-sm text-text-secondary mt-2">
              不适合场景：
            </p>
            <List>
              <ListItem>
                <p className="text-sm">• 需要高速下载/上传</p>
              </ListItem>
              <ListItem>
                <p className="text-sm">• 需要低延迟（游戏、视频通话）</p>
              </ListItem>
              <ListItem>
                <p className="text-sm">• 日常轻度使用</p>
              </ListItem>
            </List>
          </Paper>
        </TabPanel>
      </DialogContent>
    </Dialog>
  )
}
