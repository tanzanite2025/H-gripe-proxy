import type { VisibleSectionKind } from './types'

type SectionCopy = {
  title: string
  description?: string
}

export const GROUP_SECTION_COPY: Record<VisibleSectionKind, SectionCopy> = {
  manual: {
    title: '出口选择',
  },
  strategy: {
    title: '策略池',
    description:
      '这里只显示你手动加入策略池的成员，池内节点由策略自动切换，不在池内单独点选节点。',
  },
}

export const GLOBAL_SELECTOR_SECTION_COPY: Record<
  VisibleSectionKind,
  SectionCopy
> = {
  manual: {
    title: '出口选择',
    description: '这里显示所有已提取的真实节点，直接选择后即可作为当前出口。',
  },
  strategy: {
    title: '策略池',
    description:
      '这里仅显示软件维护的策略池。先选中策略池，再由池内策略在你手动添加的成员之间自动选择出口。',
  },
}

export const MANUAL_PAGE_SECTION_COPY: SectionCopy = {
  title: '节点组',
  description: '这里只显示可以直接决定出口的主组。',
}
