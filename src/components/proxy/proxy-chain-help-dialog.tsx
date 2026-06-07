import {
  AlertTriangle as WarningIcon,
  CheckCircle as CheckIcon,
  HelpCircle as HelpIcon,
  Info as InfoIcon,
  X as CloseIcon,
} from 'lucide-react'
import { useState, type ReactNode, type SyntheticEvent } from 'react'

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
  children?: ReactNode
  index: number
  value: number
}

interface HelpListItemData {
  tone: 'check' | 'warning' | 'info'
  title: string
  description: string
}

interface ExampleCardProps {
  title: string
  scene: string
  nodes: Array<{
    role: string
    color: 'success' | 'primary' | 'warning'
    description: string
  }>
  alertSeverity: 'info' | 'warning'
  summary: string
}

interface FaqCardProps {
  question: string
  intro: string
  bullets: string[]
}

const iconMap = {
  check: <CheckIcon className="text-success h-4 w-4" />,
  warning: <WarningIcon className="text-warning h-4 w-4" />,
  info: <InfoIcon className="text-info h-4 w-4" />,
}

const TabPanel = ({ children, value, index }: TabPanelProps) => {
  return (
    <div role="tabpanel" hidden={value !== index}>
      {value === index && <div className="py-4">{children}</div>}
    </div>
  )
}

const renderHelpList = (items: HelpListItemData[]) => (
  <List>
    {items.map((item) => (
      <ListItem key={item.title}>
        <ListItemIcon>{iconMap[item.tone]}</ListItemIcon>
        <ListItemText
          primary={item.title}
          secondary={item.description}
        />
      </ListItem>
    ))}
  </List>
)

const RoleCard = ({
  chipLabel,
  chipColor,
  description,
  hint,
}: {
  chipLabel: string
  chipColor: 'success' | 'primary' | 'warning'
  description: string
  hint: string
}) => (
  <Paper variant="outlined" className="mb-2 p-4 last:mb-0">
    <div className="mb-2 flex items-center gap-2">
      <Chip label={chipLabel} size="small" color={chipColor} />
      <p className="text-sm">{description}</p>
    </div>
    <p className="text-xs text-text-secondary">{hint}</p>
  </Paper>
)

const ExampleCard = ({
  title,
  scene,
  nodes,
  alertSeverity,
  summary,
}: ExampleCardProps) => (
  <Paper variant="outlined" className="mb-4 p-4 last:mb-0">
    <h6 className="mb-2 text-sm font-semibold">{title}</h6>
    <p className="mb-2 text-sm">
      <strong>场景:</strong> {scene}
    </p>
    <div className="space-y-2">
      {nodes.map((node) => (
        <div key={`${title}-${node.role}`} className="flex items-center gap-2">
          <Chip label={node.role} size="small" color={node.color} />
          <p className="text-sm">{node.description}</p>
        </div>
      ))}
    </div>
    <Alert severity={alertSeverity} className="mt-3">
      <p className="text-xs">{summary}</p>
    </Alert>
  </Paper>
)

const FaqCard = ({ question, intro, bullets }: FaqCardProps) => (
  <Paper variant="outlined" className="mb-4 p-4 last:mb-0">
    <h6 className="mb-2 text-sm font-semibold">{question}</h6>
    <p className="text-sm text-text-secondary">{intro}</p>
    <List>
      {bullets.map((bullet) => (
        <ListItem key={`${question}-${bullet}`} className="pl-0">
          <p className="text-sm">{bullet}</p>
        </ListItem>
      ))}
    </List>
  </Paper>
)

export const ProxyChainHelpDialog = ({
  open,
  onClose,
}: ProxyChainHelpDialogProps) => {
  const [tabValue, setTabValue] = useState(0)

  const handleTabChange = (
    _event: SyntheticEvent,
    newValue: string | number,
  ) => {
    setTabValue(Number(newValue))
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
        <Tabs
          value={tabValue}
          onChange={handleTabChange}
          className="border-b border-divider"
        >
          <Tab label="是什么" />
          <Tab label="怎么配置" />
          <Tab label="示例" />
          <Tab label="建议" />
          <Tab label="FAQ" />
        </Tabs>

        <TabPanel value={tabValue} index={0}>
          <h6 className="mb-2 text-base font-semibold">什么是代理链？</h6>
          <p className="mb-4 text-sm">
            代理链会把多个代理节点串起来，流量会依次经过入口节点、中间节点和出口节点，最后再访问目标站点。
          </p>

          <Alert severity="info" className="mb-4">
            <p className="text-sm">
              <strong>工作路径:</strong>
              <br />
              设备 → 入口节点 → 中间节点 → 出口节点 → 目标网站
            </p>
          </Alert>

          <h6 className="mb-2 mt-4 text-sm font-semibold">主要优势</h6>
          {renderHelpList([
            {
              tone: 'check',
              title: '增强隐私保护',
              description: '多层转发会增加追踪真实出口的难度。',
            },
            {
              tone: 'check',
              title: '绕过单一区域限制',
              description: '可以把出口节点放在目标地区，适合访问区域内容。',
            },
            {
              tone: 'check',
              title: '出口更灵活',
              description: '入口和出口可以分离，方便按用途组合线路。',
            },
          ])}

          <h6 className="mb-2 mt-4 text-sm font-semibold">主要代价</h6>
          {renderHelpList([
            {
              tone: 'warning',
              title: '延迟会增加',
              description: '每多一个节点，链路就会更长。',
            },
            {
              tone: 'warning',
              title: '速度受最慢节点限制',
              description: '整条链的吞吐通常由最慢或最不稳定的节点决定。',
            },
            {
              tone: 'warning',
              title: '稳定性更依赖整体',
              description: '任意一个节点出问题，都可能导致整条链不可用。',
            },
          ])}
        </TabPanel>

        <TabPanel value={tabValue} index={1}>
          <h6 className="mb-2 text-base font-semibold">如何配置代理链？</h6>

          <Alert severity="info" className="mb-4">
            代理链至少需要 <strong>2 个节点</strong>，也就是入口节点和出口节点。
          </Alert>

          <h6 className="mb-2 text-sm font-semibold">配置步骤</h6>
          <List>
            {[
              ['1', '启用链式模式', '打开代理链功能入口。'],
              ['2', '选择作用范围', '在规则模式下先确认要作用的代理组。'],
              ['3', '按顺序添加节点', '依次点击节点，形成入口到出口的路径。'],
              ['4', '拖拽调整顺序', '拖动卡片可以改变节点位置和角色。'],
              ['5', '点击连接', '确认链路后应用到当前运行态。'],
            ].map(([step, title, description]) => (
              <ListItem key={step}>
                <ListItemIcon>
                  <Chip label={step} size="small" color="primary" />
                </ListItemIcon>
                <ListItemText primary={title} secondary={description} />
              </ListItem>
            ))}
          </List>

          <div className="my-4 border-t border-divider" />

          <h6 className="mb-2 text-sm font-semibold">节点角色说明</h6>
          <RoleCard
            chipLabel="入口节点"
            chipColor="success"
            description="设备首先连接到这里，入口质量会直接影响体验。"
            hint="建议选择距离近、延迟低、稳定性高的节点。"
          />
          <RoleCard
            chipLabel="中间节点"
            chipColor="primary"
            description="用于继续转发流量，通常承担隐私增强或中转作用。"
            hint="不是必须，但在需要额外中转时很有价值。"
          />
          <RoleCard
            chipLabel="出口节点"
            chipColor="warning"
            description="目标站点最终看到的是这个节点的出口 IP。"
            hint="建议选择目标地区或目标业务更友好的节点。"
          />
        </TabPanel>

        <TabPanel value={tabValue} index={2}>
          <h6 className="mb-2 text-base font-semibold">配置示例</h6>

          <ExampleCard
            title="示例 1: 双跳基础链"
            scene="日常访问海外站点，希望在隐私和速度之间平衡。"
            nodes={[
              {
                role: '入口',
                color: 'success',
                description: '香港节点，作为低延迟入口。',
              },
              {
                role: '出口',
                color: 'warning',
                description: '美国节点，作为目标地区出口。',
              },
            ]}
            alertSeverity="info"
            summary="预估延迟 150-250ms，适合大多数日常使用。"
          />

          <ExampleCard
            title="示例 2: 三跳隐私链"
            scene="更看重隐私，希望多一层中转。"
            nodes={[
              {
                role: '入口',
                color: 'success',
                description: '香港节点，保证入口体验。',
              },
              {
                role: '中间',
                color: 'primary',
                description: '新加坡节点，承担中转。',
              },
              {
                role: '出口',
                color: 'warning',
                description: '美国节点，提供最终出口。',
              },
            ]}
            alertSeverity="warning"
            summary="预估延迟 250-400ms，更适合隐私优先的场景。"
          />

          <ExampleCard
            title="示例 3: 地区定向链"
            scene="访问特定区域内容，希望出口定位更准确。"
            nodes={[
              {
                role: '入口',
                color: 'success',
                description: '日本节点，作为邻近入口。',
              },
              {
                role: '出口',
                color: 'warning',
                description: '台湾节点，作为目标地区出口。',
              },
            ]}
            alertSeverity="info"
            summary="预估延迟 100-180ms，适合目标地区明确的访问场景。"
          />
        </TabPanel>

        <TabPanel value={tabValue} index={3}>
          <h6 className="mb-2 text-base font-semibold">最佳实践</h6>

          <h6 className="mb-2 mt-4 text-sm font-semibold">节点选择建议</h6>
          {renderHelpList([
            {
              tone: 'info',
              title: '入口节点优先低延迟',
              description: '入口节点离设备越近，整体体验通常越稳。',
            },
            {
              tone: 'info',
              title: '出口节点贴合目标地区',
              description: '出口位置应根据访问内容和业务地区来定。',
            },
            {
              tone: 'info',
              title: '不要堆太多节点',
              description: '2-3 个节点通常足够，再多只会明显拉高延迟。',
            },
          ])}

          <div className="my-4 border-t border-divider" />

          <h6 className="mb-2 text-sm font-semibold">性能优化建议</h6>
          {renderHelpList([
            {
              tone: 'check',
              title: '先看节点延迟再组链',
              description: '优先用延迟和丢包表现更好的节点做入口。',
            },
            {
              tone: 'check',
              title: '尽量避免跨大洲乱跳',
              description: '大跨度中转一般只会增加时延和失败概率。',
            },
            {
              tone: 'check',
              title: '按用途保留几条固定链',
              description: '为日常、隐私、特定地区访问分别准备不同组合更稳。',
            },
          ])}

          <div className="my-4 border-t border-divider" />

          <h6 className="mb-2 text-sm font-semibold">安全建议</h6>
          {renderHelpList([
            {
              tone: 'warning',
              title: '不要混入来源不明的节点',
              description: '任何一个不可信节点都可能破坏整条链的可靠性。',
            },
            {
              tone: 'warning',
              title: '定期复测节点质量',
              description: '同一条链长期使用后，节点状态可能已经变化。',
            },
            {
              tone: 'warning',
              title: '配合 DNS 防泄漏策略',
              description: '代理链本身不能替代 DNS 防护，两者应同时配置。',
            },
          ])}
        </TabPanel>

        <TabPanel value={tabValue} index={4}>
          <h6 className="mb-2 text-base font-semibold">常见问题</h6>

          <FaqCard
            question="Q: 代理链连接失败怎么办？"
            intro="可以优先检查下面几项："
            bullets={[
              '确保所有节点本身可用，延迟测试不是全超时。',
              '确认链上至少有 2 个节点。',
              '尝试更换节点顺序，尤其是入口节点。',
              '检查当前代理组和运行态是否允许切换。',
            ]}
          />

          <FaqCard
            question="Q: 代理链速度很慢怎么办？"
            intro="通常可以从这些方向优化："
            bullets={[
              '把节点数量控制在 2-3 个。',
              '优先替换最慢的入口或出口节点。',
              '减少跨大洲跳转。',
              '为不同用途拆成不同链路，不要一条链包打天下。',
            ]}
          />

          <FaqCard
            question="Q: 代理链和普通代理有什么区别？"
            intro="核心差异在于路径长度和控制粒度："
            bullets={[
              '普通代理: 设备 → 代理 → 目标网站。',
              '代理链: 设备 → 代理 1 → 代理 2 → ... → 目标网站。',
              '代理链隐私更强，但延迟和复杂度也更高。',
            ]}
          />

          <FaqCard
            question="Q: 配置会保存到哪里？"
            intro="当前代理链配置会保存在本地，用于下次恢复。"
            bullets={[
              '清空链路会清除当前链配置。',
              '切换设备不会自动同步。',
              '重要链路建议手动记录节点顺序和用途。',
            ]}
          />

          <FaqCard
            question="Q: 代理链适合什么场景？"
            intro="更适合这些需求："
            bullets={[
              '希望把入口延迟和出口地区分开控制。',
              '需要更强的中转隐私。',
              '需要定向访问特定区域内容。',
              '不适合极度追求低延迟的游戏或实时通话场景。',
            ]}
          />
        </TabPanel>
      </DialogContent>
    </Dialog>
  )
}
