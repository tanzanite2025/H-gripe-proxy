import { Button } from '@/components/tailwind/Button'

interface SectionHeaderProps {
  title: string
  description: string
  actionLabel: string
  onAction: () => void
}

export function SectionHeader({
  title,
  description,
  actionLabel,
  onAction,
}: SectionHeaderProps) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div>
        <p className="font-semibold">{title}</p>
        <p className="mt-1 text-sm text-gray-500">{description}</p>
      </div>
      <Button size="small" variant="outlined" onClick={onAction}>
        {actionLabel}
      </Button>
    </div>
  )
}
