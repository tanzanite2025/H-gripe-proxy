import {
  AlertTriangle as WarningIcon,
  CheckCircle as CheckIcon,
  Info as InfoIcon,
} from 'lucide-react'
import type { ReactNode } from 'react'

import { Alert } from '@/components/tailwind/Alert'
import { Chip } from '@/components/tailwind/Chip'
import {
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
} from '@/components/tailwind/List'
import { Paper } from '@/components/tailwind/Paper'

import type {
  ExampleCardData,
  FaqCardData,
  HelpItemTone,
  HelpListItemData,
  RoleCardData,
} from './types'

interface TabPanelProps {
  children?: ReactNode
  index: number
  value: number
}

const iconMap: Record<HelpItemTone, ReactNode> = {
  check: <CheckIcon className="text-success h-4 w-4" />,
  warning: <WarningIcon className="text-warning h-4 w-4" />,
  info: <InfoIcon className="text-info h-4 w-4" />,
}

export const ProxyChainHelpTabPanel = ({
  children,
  value,
  index,
}: TabPanelProps) => {
  return (
    <div role="tabpanel" hidden={value !== index}>
      {value === index && <div className="py-4">{children}</div>}
    </div>
  )
}

export const HelpList = ({ items }: { items: HelpListItemData[] }) => {
  return (
    <List>
      {items.map((item) => (
        <ListItem key={item.title}>
          <ListItemIcon>{iconMap[item.tone]}</ListItemIcon>
          <ListItemText primary={item.title} secondary={item.description} />
        </ListItem>
      ))}
    </List>
  )
}

export const RoleCard = ({
  chipLabel,
  chipColor,
  description,
  hint,
}: RoleCardData) => {
  return (
    <Paper variant="outlined" className="mb-2 p-4 last:mb-0">
      <div className="mb-2 flex items-center gap-2">
        <Chip label={chipLabel} size="small" color={chipColor} />
        <p className="text-sm">{description}</p>
      </div>
      <p className="text-xs text-text-secondary">{hint}</p>
    </Paper>
  )
}

export const ExampleCard = ({
  title,
  scene,
  nodes,
  alertSeverity,
  summary,
}: ExampleCardData) => {
  return (
    <Paper variant="outlined" className="mb-4 p-4 last:mb-0">
      <h6 className="mb-2 text-sm font-semibold">{title}</h6>
      <p className="mb-2 text-sm">
        <strong>场景：</strong>
        {scene}
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
}

export const FaqCard = ({ question, intro, bullets }: FaqCardData) => {
  return (
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
}
