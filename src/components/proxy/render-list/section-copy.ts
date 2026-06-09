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
      '这里只显示你手动加入策略池的成员，池内节点由策略自行切换，不在这里做单节点直选。',
  },
}

export const GLOBAL_SELECTOR_SECTION_COPY: Record<'manual', SectionCopy> = {
  manual: {
    title: '节点列表',
    description: '这里只显示已经解析出来的真实节点，选中后即可作为当前出口使用。',
  },
}

export const MANUAL_PAGE_SECTION_COPY: SectionCopy = {
  title: '节点组',
  description: '这里只显示可以直接决定出口的主组。',
}
