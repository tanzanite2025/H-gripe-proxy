import type {
  ExampleCardData,
  FaqCardData,
  HelpListItemData,
  RoleCardData,
  SetupStepData,
} from './types'

export const PROXY_CHAIN_HELP_TITLE = '代理链使用指南'

export const PROXY_CHAIN_HELP_TABS = [
  '是什么',
  '怎么配置',
  '示例',
  '建议',
  'FAQ',
] as const

export const OVERVIEW_BENEFITS: HelpListItemData[] = [
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
]

export const OVERVIEW_TRADEOFFS: HelpListItemData[] = [
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
]

export const SETUP_STEPS: SetupStepData[] = [
  {
    step: '1',
    title: '启用链式模式',
    description: '打开代理链功能入口。',
  },
  {
    step: '2',
    title: '选择作用范围',
    description: '先确认这条代理链要作用到哪个目标分组。',
  },
  {
    step: '3',
    title: '按顺序添加节点',
    description: '依次点击节点，形成入口到出口的路径。',
  },
  {
    step: '4',
    title: '拖拽调整顺序',
    description: '拖动卡片可以改变节点位置和角色。',
  },
  {
    step: '5',
    title: '点击连接',
    description: '确认链路后应用到当前运行态。',
  },
]

export const SETUP_ROLES: RoleCardData[] = [
  {
    chipLabel: '入口节点',
    chipColor: 'success',
    description: '设备首先连接到这里，入口质量会直接影响体验。',
    hint: '建议选择距离近、延迟低、稳定性高的节点。',
  },
  {
    chipLabel: '中间节点',
    chipColor: 'primary',
    description: '用于继续转发流量，通常承担隐私增强或中转作用。',
    hint: '不是必须，但在需要额外中转时很有价值。',
  },
  {
    chipLabel: '出口节点',
    chipColor: 'warning',
    description: '目标站点最终看到的是这个节点的出口 IP。',
    hint: '建议选择目标地区或目标业务更友好的节点。',
  },
]

export const EXAMPLE_CARDS: ExampleCardData[] = [
  {
    title: '示例 1：双跳基础链',
    scene: '日常访问海外站点，希望在隐私和速度之间平衡。',
    nodes: [
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
    ],
    alertSeverity: 'info',
    summary: '预计延迟 150-250ms，适合大多数日常使用。',
  },
  {
    title: '示例 2：三跳隐私链',
    scene: '更看重隐私，希望多一层中转。',
    nodes: [
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
    ],
    alertSeverity: 'warning',
    summary: '预计延迟 250-400ms，更适合隐私优先的场景。',
  },
  {
    title: '示例 3：地区定向链',
    scene: '访问特定区域内容，希望出口定位更准确。',
    nodes: [
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
    ],
    alertSeverity: 'info',
    summary: '预计延迟 100-180ms，适合目标地区明确的访问场景。',
  },
]

export const BEST_PRACTICE_SELECTION: HelpListItemData[] = [
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
]

export const BEST_PRACTICE_PERFORMANCE: HelpListItemData[] = [
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
]

export const BEST_PRACTICE_SECURITY: HelpListItemData[] = [
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
]

export const FAQ_CARDS: FaqCardData[] = [
  {
    question: 'Q：代理链连接失败怎么办？',
    intro: '可以优先检查下面几项：',
    bullets: [
      '确保所有节点本身可用，延迟测试不是全超时。',
      '确认链上至少有 2 个节点。',
      '尝试更换节点顺序，尤其是入口节点。',
      '检查当前代理组和运行态是否允许切换。',
    ],
  },
  {
    question: 'Q：代理链速度很慢怎么办？',
    intro: '通常可以从这些方向优化：',
    bullets: [
      '把节点数量控制在 2-3 个。',
      '优先替换最慢的入口或出口节点。',
      '减少跨大洲跳转。',
      '为不同用途拆成不同链路，不要一条链包打天下。',
    ],
  },
  {
    question: 'Q：代理链和普通代理有什么区别？',
    intro: '核心差异在于路径长度和控制粒度：',
    bullets: [
      '普通代理：设备 -> 代理 -> 目标网站。',
      '代理链：设备 -> 代理 1 -> 代理 2 -> ... -> 目标网站。',
      '代理链隐私更强，但延迟和复杂度也更高。',
    ],
  },
  {
    question: 'Q：配置会保存到哪里？',
    intro: '当前代理链配置会保存在本地，用于下次恢复。',
    bullets: [
      '清空链路会清除当前链配置。',
      '切换设备不会自动同步。',
      '重要链路建议手动记录节点顺序和用途。',
    ],
  },
  {
    question: 'Q：代理链适合什么场景？',
    intro: '更适合这些需求：',
    bullets: [
      '希望把入口延迟和出口地区分开控制。',
      '需要更强的中转隐私。',
      '需要定向访问特定区域内容。',
      '不适合极度追求低延迟的游戏或实时通话场景。',
    ],
  },
]
