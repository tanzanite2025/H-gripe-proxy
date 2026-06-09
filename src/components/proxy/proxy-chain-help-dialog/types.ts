export type HelpItemTone = 'check' | 'warning' | 'info'
export type HelpChipColor = 'success' | 'primary' | 'warning'
export type HelpAlertSeverity = 'info' | 'warning'

export interface HelpListItemData {
  tone: HelpItemTone
  title: string
  description: string
}

export interface ExampleNodeData {
  role: string
  color: HelpChipColor
  description: string
}

export interface ExampleCardData {
  title: string
  scene: string
  nodes: ExampleNodeData[]
  alertSeverity: HelpAlertSeverity
  summary: string
}

export interface FaqCardData {
  question: string
  intro: string
  bullets: string[]
}

export interface RoleCardData {
  chipLabel: string
  chipColor: HelpChipColor
  description: string
  hint: string
}

export interface SetupStepData {
  step: string
  title: string
  description: string
}
